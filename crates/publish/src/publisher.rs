use std::{
  collections::{HashMap, HashSet},
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use yourdocs_core::{
  AssetId, AssetStore, Broadcast, ContentStore, Document, Event, Page, PageId, PageKind, Release, ReleaseStatus, RevisionId, UserId, effective_redirects, expand, extract_links, header_section,
  original_key, page_urls, term_anchors, term_refs, variant_key,
};
use yourdocs_render::{Pool, Shell, highlight_css};
use yourdocs_store::{DatabaseConnection, add_release_pages, assets_by_ids, create_release, list_pages, list_redirects, load_library, load_settings, set_release_status};

use crate::{
  PublishError, generated,
  markdown::{Site, page_markdown},
  search,
  site::{breadcrumbs, header_items, image_asset_ids, markdown_path, nav_tree, output_path, previous_next},
  target::{OutputFile, PublishTarget},
};

/// The browser-side files every page links to; embedded in the binary and written into each release.
pub struct Bundles {
  pub client_js: Vec<u8>,
  pub site_css: Vec<u8>,
}

/// Settings that come from the deployment rather than the content.
pub struct PublishConfig {
  /// The public address of the site without a trailing slash, for the sitemap and llms.txt.
  pub site_url: String,
  /// The Pagefind executable (a path or a name on `PATH`).
  pub pagefind: String,
}

/// How many pages are in V8 at once. Below the pool's queue size, so a big site never gets "busy" from its own publish.
const RENDER_CONCURRENCY: usize = 16;

/// Everything a `Publisher` is made of.
pub struct PublisherParts<C, A, B> {
  pub db: DatabaseConnection,
  pub content: C,
  pub assets: Arc<A>,
  pub pool: Arc<Pool>,
  pub target: PublishTarget,
  pub bundles: Bundles,
  pub config: PublishConfig,
  pub broadcast: B,
}

/// Builds releases: renders every page, assembles the files and hands them to the target.
pub struct Publisher<C: ContentStore, A: AssetStore, B: Broadcast> {
  db: DatabaseConnection,
  content: C,
  assets: Arc<A>,
  pool: Arc<Pool>,
  target: PublishTarget,
  bundles: Bundles,
  config: PublishConfig,
  broadcast: B,
  // Releases are built one at a time; a second publish waits for the first.
  running: tokio::sync::Mutex<()>,
  /// How many pages went through V8 since startup; pages whose output could be reused are not counted.
  renders: Arc<AtomicUsize>,
}

struct PageSource {
  page: Page,
  doc: Document,
  revision: RevisionId,
  /// When the revision being published was saved: the page's "last updated".
  updated: jiff::Timestamp,
}

impl<C: ContentStore, A: AssetStore, B: Broadcast> Publisher<C, A, B> {
  pub fn new(parts: PublisherParts<C, A, B>) -> Self {
    Self {
      db: parts.db,
      content: parts.content,
      assets: parts.assets,
      pool: parts.pool,
      target: parts.target,
      bundles: parts.bundles,
      config: parts.config,
      broadcast: parts.broadcast,
      running: tokio::sync::Mutex::new(()),
      renders: Arc::new(AtomicUsize::new(0)),
    }
  }

  /// How many pages have been rendered in V8 since startup. A publish that changes one page adds one.
  pub fn renders(&self) -> usize {
    self.renders.load(Ordering::Relaxed)
  }

  pub fn target(&self) -> &PublishTarget {
    &self.target
  }

  /// The public address of the site, without a trailing slash.
  pub fn site_url(&self) -> &str {
    &self.config.site_url
  }

  /// Publishes every page's latest saved draft as a new release and makes it live.
  pub async fn publish(&self, user: UserId) -> Result<Release, PublishError> {
    let _turn = self.running.lock().await;

    // Nothing is created, rendered or written until the site passes its checks.
    let problems = crate::changes::check(&self.db, &self.pool).await?;
    if !problems.is_empty() {
      return Err(PublishError::Blocked(problems));
    }

    let tree_pages = list_pages(&self.db).await?;
    // Links are navigation only: nothing to render, nothing in the sitemap.
    let sources = self.sources(tree_pages.iter().filter(|p| p.kind == PageKind::Page).cloned().collect(), user).await?;

    let rules = effective_redirects(&list_redirects(&self.db).await?, &tree_pages);
    // Frozen with the release: the next "what changed" comparison reads them back.
    let release = create_release(
      &self.db,
      user,
      serde_json::to_string(&tree_pages).map_err(anyhow::Error::from)?,
      serde_json::to_string(&rules).map_err(anyhow::Error::from)?,
    )
    .await?;

    match self.build_and_deploy(&release, &tree_pages, &rules, &sources).await {
      Ok(()) => {
        // The site is already live: a failure here leaves the record behind it, which the log has to say.
        if let Err(error) = set_release_status(&self.db, release.id, ReleaseStatus::Live).await {
          tracing::error!(release_id = %release.id, %error, "the release is live but could not be marked so");
          return Err(error.into());
        }
        tracing::info!(release_id = %release.id, pages = sources.len(), "release is live");
        self.broadcast.send(Event::Published { release: release.id });

        Ok(Release {
          status: ReleaseStatus::Live,
          page_count: sources.len() as u32,
          ..release
        })
      }
      Err(error) => {
        tracing::error!(release_id = %release.id, %error, "release failed");
        set_release_status(&self.db, release.id, ReleaseStatus::Failed).await?;

        Err(error)
      }
    }
  }

  /// Each page's latest revision, which is always what its draft holds. A page nobody has edited gets an empty one.
  async fn sources(&self, pages: Vec<Page>, user: UserId) -> Result<Vec<PageSource>, PublishError> {
    let mut sources = Vec::with_capacity(pages.len());
    let library = load_library(&self.db).await?;

    for page in pages {
      let revision = self.content.ensure_revision(page.id, user).await?;
      sources.push(PageSource {
        page,
        // Variables and snippets are written into the page here, so everything after (the render, the Markdown, the cache key)
        // sees the page as readers do, and a changed entry changes exactly the pages that use it.
        doc: expand(&revision.doc, &library).0,
        revision: revision.id,
        updated: revision.created_at,
      });
    }

    Ok(sources)
  }

  /// Renders everything, then deploys. Nothing reaches the target unless every page rendered.
  async fn build_and_deploy(&self, release: &Release, tree_pages: &[Page], rules: &[(String, String)], sources: &[PageSource]) -> Result<(), PublishError> {
    let pages: Vec<Page> = sources.iter().map(|s| s.page.clone()).collect();
    let urls = page_urls(tree_pages);
    let header = header_items(tree_pages, &urls);
    // What link cards and other components show about the pages they point to; a rename or new description reaches them at the next publish.
    let url_values: serde_json::Map<String, Value> = urls.iter().map(|(id, url)| (id.to_string(), Value::String(url.clone()))).collect();
    let page_info = tree_pages
      .iter()
      .filter(|p| p.kind == PageKind::Page)
      .map(|p| (p.id.to_string(), json!({ "title": p.title, "description": p.description })))
      .collect::<serde_json::Map<String, Value>>();
    // The glossary: every term in order, where its entry is, and where the Glossary page itself lives (never on top of a real page).
    let library = load_library(&self.db).await?;
    let mut glossary_terms: Vec<String> = library.terms.keys().cloned().collect();
    glossary_terms.sort_by_key(|t| t.to_lowercase());
    let anchors = term_anchors(&glossary_terms);
    let glossary_url = if urls.values().any(|u| u == "/glossary/") { "/glossary-terms/" } else { "/glossary/" };
    let settings = load_settings(&self.db).await?;
    let site_name = settings.site_name.clone();
    let today = jiff::Zoned::now().date().to_string();

    let (asset_files, asset_meta) = self.asset_files(sources, [settings.logo_asset_id, settings.logo_dark_asset_id].into_iter().flatten()).await?;
    let logo = settings.logo_asset_id.and_then(|id| asset_meta.get(&id.to_string()).cloned());
    let logo_dark = settings.logo_dark_asset_id.and_then(|id| asset_meta.get(&id.to_string()).cloned());
    let site_ctx = json!({ "name": site_name, "logo": logo, "logoDark": logo_dark, "headerLinks": settings.header_links, "footer": settings.footer });
    let theme_css = settings.theme_css();

    let bundle_files = bundle_files(&self.bundles);
    let stylesheets: Vec<String> = bundle_files.iter().filter(|(name, _)| name.ends_with(".css")).map(|(name, _)| format!("/{name}")).collect();
    let script = bundle_files.iter().find(|(name, _)| name.ends_with(".js")).map(|(name, _)| format!("/{name}")).unwrap_or_default();

    // The jobs own their data: borrowed closures here make the future too lifetime-generic for the HTTP handler that awaits it.
    let jobs: Vec<RenderJob> = sources
      .iter()
      .map(|source| {
        let nav = nav_tree(tree_pages, &urls, Some(source.page.id));
        let (previous, next) = previous_next(&nav, source.page.id);
        // Only what the page refers to goes into its context: renaming one page must not change the input of every other, or nothing
        // could be reused.
        let referenced: HashSet<String> = extract_links(&source.doc).into_iter().map(|l| l.target.to_string()).collect();
        // The date only matters to components that count down to one, and it changes every day.
        // Only the terms this page marks go into its context, so defining another term leaves it alone.
        let terms: serde_json::Map<String, Value> = term_refs(&source.doc)
          .into_iter()
          .filter_map(|t| Some((t.clone(), json!({ "definition": library.terms.get(&t)?, "href": format!("{glossary_url}#{}", anchors.get(&t)?) }))))
          .collect();
        let today = if serde_json::to_string(&source.doc.0).is_ok_and(|json| json.contains("\"newUntil\"")) {
          today.as_str()
        } else {
          ""
        };

        RenderJob {
          pool: self.pool.clone(),
          input: json!({
            "doc": source.doc,
            "context": {
              "site": site_ctx,
              "page": { "id": source.page.id, "title": source.page.title, "description": source.page.description },
              "nav": nav,
              "breadcrumbs": breadcrumbs(tree_pages, &urls, source.page.id),
              "previous": previous,
              "next": next,
              "updated": source.updated.to_zoned(jiff::tz::TimeZone::UTC).date().to_string(),
              "markdown": format!("/{}", markdown_path(&urls[&source.page.id])),
              "header": header,
              "section": header_section(tree_pages, source.page.id),
              "urls": only(&url_values, &referenced),
              "pages": only(&page_info, &referenced),
              "assets": source_assets(&source.doc, &asset_meta),
              "terms": terms,
              "today": today,
            },
          }),
          title: source.page.title.clone(),
          description: source.page.description.clone(),
          site_name: site_name.clone(),
          stylesheets: stylesheets.clone(),
          script: script.clone(),
          theme_css: theme_css.clone(),
          page_id: source.page.id,
          revision: source.revision,
          path: output_path(&urls[&source.page.id]),
        }
      })
      .collect();

    let rendered: Vec<(OutputFile, (PageId, RevisionId, String))> = stream::iter(jobs.into_iter().map(|job| render_job(job, self.assets.clone(), self.renders.clone())))
      .buffer_unordered(RENDER_CONCURRENCY)
      .try_collect()
      .await?;

    let (mut files, hashes): (Vec<_>, Vec<_>) = rendered.into_iter().unzip();

    let listed: Vec<(&Page, &str)> = pages.iter().map(|p| (p, urls[&p.id].as_str())).collect();
    // The site root goes to the first top-level page. Positions only order siblings: the lowest one in the whole tree can be a child's.
    let home = pages.iter().find(|p| p.parent_id.is_none()).or(pages.first());
    let site_url = self.config.site_url.as_str();
    let generated = [
      ("sitemap.xml", generated::sitemap(site_url, &listed)),
      ("robots.txt", generated::robots(site_url)),
      ("llms.txt", generated::llms_txt(&site_name, site_url, &listed)),
      ("_redirects", generated::redirects(home.map(|p| urls[&p.id].as_str()), rules)),
    ];
    files.extend(generated.into_iter().map(|(path, text)| OutputFile {
      path: path.into(),
      bytes: text.into_bytes(),
    }));

    // Every page also as Markdown, next to its HTML: `/guide/install/` has `/guide/install.md`.
    let url_strings: HashMap<String, String> = urls.iter().map(|(id, url)| (id.to_string(), url.clone())).collect();
    let titles: HashMap<String, String> = tree_pages.iter().map(|p| (p.id.to_string(), p.title.clone())).collect();
    let asset_paths: HashMap<String, String> = asset_meta
      .iter()
      .filter_map(|(id, meta)| Some((id.clone(), meta["src"].as_str()?.trim_start_matches('/').to_string())))
      .collect();
    let site = Site {
      url: site_url,
      urls: &url_strings,
      titles: &titles,
      assets: &asset_paths,
    };
    files.extend(sources.iter().map(|s| OutputFile {
      path: markdown_path(&urls[&s.page.id]),
      bytes: page_markdown(&s.page.title, s.page.description.as_deref(), &s.doc, &site).into_bytes(),
    }));

    if !glossary_terms.is_empty() {
      let blocks: Vec<Value> = glossary_terms
        .iter()
        .flat_map(|t| {
          [
            json!({ "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": t }] }),
            json!({ "type": "paragraph", "content": [{ "type": "text", "text": library.terms[t] }] }),
          ]
        })
        .collect();
      let glossary = RenderJob {
        pool: self.pool.clone(),
        input: json!({
          "doc": { "type": "doc", "content": blocks },
          // A fixed ID keeps the page searchable and its render cacheable.
          "context": { "site": site_ctx, "page": { "id": "00000000-0000-7000-8000-00000000610c", "title": "Glossary" }, "nav": nav_tree(tree_pages, &urls, None), "header": header, "urls": {}, "assets": {}, "today": "" },
        }),
        title: "Glossary".into(),
        description: None,
        site_name: site_name.clone(),
        stylesheets: stylesheets.clone(),
        script: script.clone(),
        theme_css: theme_css.clone(),
        page_id: PageId::new(),
        revision: RevisionId::new(),
        path: output_path(glossary_url),
      };
      files.push(render_job(glossary, self.assets.clone(), self.renders.clone()).await?.0);
    }

    // The page people land on for a URL that does not exist, with the site navigation to find their way.
    let not_found = RenderJob {
      pool: self.pool.clone(),
      input: json!({
        "doc": { "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "There is no page at this address. Use the navigation or the search to find what you were looking for." }] }] },
        "context": { "site": site_ctx, "page": { "title": "Page not found" }, "nav": nav_tree(tree_pages, &urls, None), "header": header, "urls": {}, "assets": {}, "today": "" },
      }),
      title: "Page not found".into(),
      description: None,
      site_name: site_name.clone(),
      stylesheets: stylesheets.clone(),
      script: script.clone(),
      theme_css: theme_css.clone(),
      page_id: PageId::new(),
      revision: RevisionId::new(),
      path: "404.html".into(),
    };
    files.push(render_job(not_found, self.assets.clone(), self.renders.clone()).await?.0);

    let html: Vec<&OutputFile> = files.iter().filter(|f| f.path.ends_with(".html")).collect();
    if let Some(index) = search::build_index(&self.config.pagefind, release.id, &html).await? {
      files.extend(index);
    }

    files.extend(asset_files);
    files.extend(bundle_files.into_iter().map(|(path, bytes)| OutputFile { path, bytes }));

    // Recorded before the switch: once the site is live, what it holds must already be known.
    add_release_pages(&self.db, release.id, hashes).await?;
    self.target.publish(release.id, files).await?;

    Ok(())
  }

  /// The image files the pages use, and what the renderer needs to know about them.
  async fn asset_files(&self, sources: &[PageSource], logos: impl Iterator<Item = AssetId>) -> Result<(Vec<OutputFile>, HashMap<String, Value>), PublishError> {
    let mut ids: Vec<_> = sources.iter().flat_map(|s| image_asset_ids(&s.doc)).chain(logos).collect();
    ids.sort();
    ids.dedup();

    let mut files = Vec::new();
    let mut meta = HashMap::new();

    for asset in assets_by_ids(&self.db, &ids).await? {
      let path = format!("_a/{}.{}", asset.id, asset.extension());
      files.push(OutputFile {
        bytes: self.assets.get(&original_key(&asset.sha256)).await?,
        path: path.clone(),
      });

      // The WebP copies for `srcset`, narrowest first; the original stays as the fallback `src`.
      let mut srcset = Vec::new();
      for variant in &asset.variants {
        let variant_path = format!("_a/{}-{}.webp", asset.id, variant.width);
        files.push(OutputFile {
          bytes: self.assets.get(&variant_key(&asset.sha256, variant.width)).await?,
          path: variant_path.clone(),
        });
        srcset.push(format!("/{variant_path} {}w", variant.width));
      }

      meta.insert(
        asset.id.to_string(),
        json!({ "src": format!("/{path}"), "width": asset.width, "height": asset.height, "srcset": srcset.join(", "), "placeholder": asset.placeholder }),
      );
    }

    Ok((files, meta))
  }
}

struct RenderJob {
  pool: Arc<Pool>,
  input: Value,
  title: String,
  description: Option<String>,
  site_name: String,
  stylesheets: Vec<String>,
  script: String,
  theme_css: String,
  page_id: PageId,
  revision: RevisionId,
  path: String,
}

/// Renders one page, or reuses what an identical render produced before. The cache is content addressed: the key is a hash of everything the
/// HTML depends on (the render input, the page's own title and description, the site name and theme, the stylesheets and script), so the same
/// key always means the same file and nothing ever has to be invalidated.
/// Part of every render cache key. The JS bundles are in the key by their hashed names, but the page shell and code highlighting are Rust:
/// a release changes the version, and the number is bumped when either changes between releases. Otherwise unchanged pages would keep
/// the old output for good.
const RENDER_FORMAT: &str = concat!(env!("CARGO_PKG_VERSION"), "/1");

async fn render_job<A: AssetStore>(job: RenderJob, cache: Arc<A>, renders: Arc<AtomicUsize>) -> Result<(OutputFile, (PageId, RevisionId, String)), PublishError> {
  let key = {
    let mut digest = Sha256::new();
    for part in [
      RENDER_FORMAT,
      &job.input.to_string(),
      &job.title,
      job.description.as_deref().unwrap_or_default(),
      &job.site_name,
      &job.stylesheets.join(","),
      &job.script,
      &job.theme_css,
    ] {
      digest.update(part.as_bytes());
      digest.update([0]);
    }
    format!("renders/{}.html", digest.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>())
  };

  let bytes = if cache.exists(&key).await? {
    cache.get(&key).await?
  } else {
    let body = job.pool.render(job.input.clone()).await.map_err(|source| PublishError::Render { page: job.title.clone(), source })?;
    renders.fetch_add(1, Ordering::Relaxed);
    let html = yourdocs_render::document(&Shell {
      title: &job.title,
      site_name: &job.site_name,
      description: job.description.as_deref(),
      body: &body,
      input: &job.input.to_string(),
      stylesheets: &job.stylesheets,
      script: &job.script,
      theme_css: &job.theme_css,
    });
    cache.put(&key, html.clone().into_bytes()).await?;

    html.into_bytes()
  };

  let hash = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();

  Ok((OutputFile { path: job.path, bytes }, (job.page_id, job.revision, hash)))
}

/// The entries of `map` for the IDs in `only`.
fn only(map: &serde_json::Map<String, Value>, ids: &HashSet<String>) -> Value {
  Value::Object(map.iter().filter(|(id, _)| ids.contains(*id)).map(|(id, v)| (id.clone(), v.clone())).collect())
}

fn source_assets(doc: &Document, meta: &HashMap<String, Value>) -> Value {
  let used: serde_json::Map<String, Value> = image_asset_ids(doc)
    .into_iter()
    .filter_map(|id| meta.get(&id.to_string()).map(|m| (id.to_string(), m.clone())))
    .collect();

  Value::Object(used)
}

/// The shared browser files with names that carry their content hash, so they can be cached for a year.
fn bundle_files(bundles: &Bundles) -> Vec<(String, Vec<u8>)> {
  let code_css = highlight_css().into_bytes();

  [("client", "js", &bundles.client_js), ("site", "css", &bundles.site_css), ("code", "css", &code_css)]
    .into_iter()
    .map(|(name, extension, bytes)| {
      let digest: String = Sha256::digest(bytes).iter().take(4).map(|b| format!("{b:02x}")).collect();
      (format!("assets/{name}-{digest}.{extension}"), bytes.clone())
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use std::{path::Path, time::Duration};

  use yourdocs_assets::LocalAssetStore;
  use yourdocs_core::{LocalBroadcast, Placement, UserId};
  use yourdocs_store::{SqlContentStore, connect, create_page};

  use super::*;
  use crate::FolderTarget;

  struct Fixture {
    publisher: Publisher<SqlContentStore, LocalAssetStore, LocalBroadcast>,
    db: DatabaseConnection,
    content: SqlContentStore,
    dir: std::path::PathBuf,
  }

  async fn fixture() -> Fixture {
    let dir = std::env::temp_dir().join(format!("yourdocs-publish-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = connect(&dir.join("test.db")).await.unwrap();

    let script = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/renderer/dist/ssr.js")).expect("run `bun run build` first");
    let pool = Arc::new(Pool::new(&script, 1, 32, Duration::from_secs(5)).unwrap());
    let content = SqlContentStore::new(db.clone());

    let publisher = Publisher::new(PublisherParts {
      db: db.clone(),
      content: content.clone(),
      assets: Arc::new(LocalAssetStore::new(&dir.join("assets")).unwrap()),
      pool,
      target: PublishTarget::Folder(FolderTarget::new(dir.join("out"))),
      bundles: Bundles {
        client_js: b"console.log(1)".to_vec(),
        site_css: b"body{}".to_vec(),
      },
      config: PublishConfig {
        site_url: "https://docs.example.com".into(),
        pagefind: std::env::var("PAGEFIND_BIN").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../node_modules/.bin/pagefind").into()),
      },
      broadcast: LocalBroadcast::new(),
    });

    Fixture { publisher, db, content, dir }
  }

  fn doc(json: serde_json::Value) -> Document {
    serde_json::from_value(json).unwrap()
  }

  fn html_files(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
      for entry in std::fs::read_dir(current).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
          stack.push(path);
        } else if path.extension().is_some_and(|e| e == "html") {
          found.push(path.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
        }
      }
    }

    found.sort();
    found
  }

  #[tokio::test]
  async fn publishes_one_html_file_per_page() {
    let f = fixture().await;
    let user = UserId::new();
    let guide = create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();
    create_page(&f.db, user, Some(guide.id), "Install", Placement::Last).await.unwrap();
    f.content
      .save_draft(
        guide.id,
        doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Hello from the guide" }] }] })),
        None,
        user,
      )
      .await
      .unwrap();

    let release = f.publisher.publish(user).await.unwrap();
    assert_eq!((release.status, release.page_count), (ReleaseStatus::Live, 2));

    let site = f.dir.join("out/site");
    assert_eq!(html_files(&site), ["404.html", "guide/index.html", "guide/install/index.html"]);

    let html = std::fs::read_to_string(site.join("guide/index.html")).unwrap();
    assert!(html.contains("Hello from the guide") && html.contains("href=\"/guide/install/\""), "{html}");

    for generated in ["sitemap.xml", "robots.txt", "llms.txt", "_redirects", "404.html", "pagefind/pagefind.js"] {
      assert!(site.join(generated).is_file(), "missing {generated}");
    }
    assert_eq!(std::fs::read_to_string(site.join("_redirects")).unwrap(), "/ /guide/ 302\n");
    assert!(std::fs::read_to_string(site.join("sitemap.xml")).unwrap().contains("https://docs.example.com/guide/install/"));
    // 404 is a page of the site but not a searchable one.
    assert!(!std::fs::read_to_string(site.join("404.html")).unwrap().contains("data-pagefind-body"));
    assert!(std::fs::read_to_string(site.join("guide/index.html")).unwrap().contains("data-pagefind-body"));
  }

  #[tokio::test]
  async fn a_failed_render_leaves_the_live_site_untouched() {
    let f = fixture().await;
    let user = UserId::new();
    let page = create_page(&f.db, user, None, "Page", Placement::Last).await.unwrap();

    f.content
      .save_draft(page.id, doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph" }] })), None, user)
      .await
      .unwrap();
    f.publisher.publish(user).await.unwrap();
    let before = std::fs::read_to_string(f.dir.join("out/site/page/index.html")).unwrap();

    // A node the renderer does not know makes the render throw.
    let draft = f.content.draft(page.id).await.unwrap().unwrap();
    f.content
      .save_draft(page.id, doc(serde_json::json!({ "type": "doc", "content": [{ "type": "bogus" }] })), draft.base_revision_id, user)
      .await
      .unwrap();

    // The checks see the unknown block before anything is rendered, and name the page.
    let blocked = f.publisher.publish(user).await;
    assert!(
      matches!(&blocked, Err(PublishError::Blocked(problems)) if problems[0].page_title == "Page" && problems[0].message.contains("unknown block")),
      "{blocked:?}"
    );
    assert_eq!(std::fs::read_to_string(f.dir.join("out/site/page/index.html")).unwrap(), before);
    // No half-written release is left behind either.
    assert_eq!(std::fs::read_dir(f.dir.join("out/releases")).unwrap().count(), 1);
  }

  #[tokio::test]
  async fn the_site_root_goes_to_the_first_top_level_page() {
    use yourdocs_core::OldUrls;
    use yourdocs_store::move_page;

    let f = fixture().await;
    let user = UserId::new();
    let guide = create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();
    let install = create_page(&f.db, user, Some(guide.id), "Install", Placement::Last).await.unwrap();
    let linux = create_page(&f.db, user, Some(guide.id), "Linux", Placement::Last).await.unwrap();
    // Dropped before its first sibling, Linux gets the lowest position of the whole tree.
    move_page(&f.db, linux.id, Some(guide.id), Placement::Before(install.id), OldUrls::Redirect).await.unwrap();

    f.publisher.publish(user).await.unwrap();
    let rules = std::fs::read_to_string(f.dir.join("out/site/_redirects")).unwrap();
    assert!(rules.starts_with("/ /guide/ 302\n"), "{rules}");
  }

  #[tokio::test]
  async fn a_move_redirects_the_old_url_unless_it_is_dropped() {
    use yourdocs_core::OldUrls;
    use yourdocs_store::move_page;

    let f = fixture().await;
    let user = UserId::new();
    let guide = create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();
    let install = create_page(&f.db, user, Some(guide.id), "Install", Placement::Last).await.unwrap();
    let setup = create_page(&f.db, user, Some(guide.id), "Setup", Placement::Last).await.unwrap();
    f.publisher.publish(user).await.unwrap();
    assert!(crate::pending(&f.db).await.unwrap().changes.is_empty());

    // A normal move keeps the old URL working, and the publish check agrees.
    move_page(&f.db, install.id, None, Placement::Last, OldUrls::Redirect).await.unwrap();
    // An Alt-drop does not: its old URL is flagged as 404 after publish.
    move_page(&f.db, setup.id, None, Placement::Last, OldUrls::Forget).await.unwrap();

    let pending = crate::pending(&f.db).await.unwrap();
    let lost: Vec<_> = pending.lost_urls.iter().map(|l| l.path.as_str()).collect();
    assert_eq!(lost, ["/guide/setup/"]);
    assert!(pending.changes.iter().any(|c| c.title == "Install" && c.redirected));

    f.publisher.publish(user).await.unwrap();
    let rules = std::fs::read_to_string(f.dir.join("out/site/_redirects")).unwrap();
    assert!(rules.contains("/guide/install/ /install/ 301\n"), "{rules}");
    assert!(!rules.contains("/guide/setup/"), "{rules}");
    assert!(crate::pending(&f.db).await.unwrap().changes.is_empty());
  }

  #[tokio::test]
  async fn promoting_a_page_with_children_scopes_its_sidebar() {
    use yourdocs_store::set_in_header;

    let f = fixture().await;
    let user = UserId::new();
    let guides = create_page(&f.db, user, None, "Guides", Placement::Last).await.unwrap();
    create_page(&f.db, user, Some(guides.id), "Install", Placement::Last).await.unwrap();
    create_page(&f.db, user, None, "About", Placement::Last).await.unwrap();
    set_in_header(&f.db, guides.id, true).await.unwrap();
    f.publisher.publish(user).await.unwrap();

    let read = |path: &str| std::fs::read_to_string(f.dir.join("out/site").join(path)).unwrap();
    // The page list only: on narrow screens the sidebar also repeats the header sections above it.
    let sidebar = |html: &str| {
      let start = html.find("class=\"site-nav\"").unwrap();
      let end = html[start..].find("</nav>").unwrap();
      let nav = &html[start..start + end];
      match nav.find("class=\"site-nav-sections\"") {
        Some(sections) => nav[sections + nav[sections..].find("</ul>").unwrap()..].to_string(),
        None => nav.to_string(),
      }
    };

    // Inside the section: its pages only. Elsewhere: the rest. Both pages see the header entry.
    let install = read("guides/install/index.html");
    assert!(sidebar(&install).contains("/guides/install/") && !sidebar(&install).contains("/about/"), "{}", sidebar(&install));
    let about = read("about/index.html");
    assert!(sidebar(&about).contains("/about/") && !sidebar(&about).contains("/guides/"), "{}", sidebar(&about));
    for html in [&install, &about] {
      assert!(html.contains("aria-label=\"Sections\"") && html.contains("href=\"/guides/\""));
    }
    assert!(install.contains("aria-current=\"page\">Guides"), "the section is marked in the header");
  }

  #[tokio::test]
  async fn renaming_a_page_updates_every_card_that_points_to_it() {
    use yourdocs_store::{rename_page, set_description};

    let f = fixture().await;
    let user = UserId::new();
    let target = create_page(&f.db, user, None, "Rollbacks", Placement::Last).await.unwrap();
    set_description(&f.db, target.id, "Put the previous release back.").await.unwrap();
    let mut sources = Vec::new();

    for title in ["First", "Second"] {
      let source = create_page(&f.db, user, None, title, Placement::Last).await.unwrap();
      let card = |kind: &str, attrs: serde_json::Value| serde_json::json!({ "type": kind, "attrs": attrs });
      let id = target.id.to_string();
      f.content
        .save_draft(
          source.id,
          doc(serde_json::json!({ "type": "doc", "content": [
            card("linkCard", serde_json::json!({ "page": id })),
            card("relatedPages", serde_json::json!({ "title": "Related", "pages": [id] })),
            card("seeAlso", serde_json::json!({ "pages": [id] })),
          ] })),
          None,
          user,
        )
        .await
        .unwrap();
      sources.push(source);
    }

    let read = |slug: &str| std::fs::read_to_string(f.dir.join("out/site").join(slug).join("index.html")).unwrap();

    f.publisher.publish(user).await.unwrap();
    for slug in ["first", "second"] {
      let html = read(slug);
      assert!(
        html.contains("Rollbacks") && html.contains("Put the previous release back.") && html.contains("href=\"/rollbacks/\""),
        "{html}"
      );
    }

    // The cards have no title of their own: renaming the page is enough.
    rename_page(&f.db, target.id, "Undo a release", yourdocs_core::OldUrls::Redirect).await.unwrap();
    f.publisher.publish(user).await.unwrap();
    for slug in ["first", "second"] {
      let html = read(slug);
      assert!(html.contains("Undo a release") && !html.contains(">Rollbacks<") && html.contains("href=\"/undo-a-release/\""), "{html}");
    }
  }

  #[tokio::test]
  async fn pages_get_breadcrumbs_neighbours_a_date_and_a_markdown_file() {
    let f = fixture().await;
    let user = UserId::new();
    let guide = create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();
    let install = create_page(&f.db, user, Some(guide.id), "Install", Placement::Last).await.unwrap();
    let linux = create_page(&f.db, user, Some(install.id), "Linux", Placement::Last).await.unwrap();
    create_page(&f.db, user, Some(guide.id), "FAQ", Placement::Last).await.unwrap();
    f.content
      .save_draft(
        linux.id,
        doc(serde_json::json!({ "type": "doc", "content": [
          { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Packages" }] },
          { "type": "paragraph", "content": [{ "type": "text", "text": "Use " }, { "type": "text", "text": "apt", "marks": [{ "type": "code" }] }] },
        ] })),
        None,
        user,
      )
      .await
      .unwrap();
    f.publisher.publish(user).await.unwrap();

    let site = f.dir.join("out/site");
    let html = std::fs::read_to_string(site.join("guide/install/linux/index.html")).unwrap();
    assert!(html.contains("aria-label=\"Breadcrumb\""), "{html}");
    assert!(html.contains("href=\"/guide/install/\">Install</a>") && html.contains("href=\"/guide/\">Guide</a>"));
    assert!(html.contains("rel=\"prev\"") && html.contains("rel=\"next\""));
    assert!(html.contains("Last updated ") && html.contains("href=\"/guide/install/linux.md\">View as Markdown"));

    assert_eq!(std::fs::read_to_string(site.join("guide/install/linux.md")).unwrap(), "# Linux\n\n## Packages\n\nUse `apt`\n");
    let llms = std::fs::read_to_string(site.join("llms.txt")).unwrap();
    assert!(llms.contains("(https://docs.example.com/guide/install/linux.md)"), "{llms}");
  }

  #[tokio::test]
  async fn images_are_published_with_webp_variants_and_a_srcset() {
    use std::io::Cursor;

    use yourdocs_assets::{inspect, process, sha256_hex};
    use yourdocs_core::{AssetStore, AssetVariant, original_key, variant_key};
    use yourdocs_store::{NewAsset, get_or_create_asset};

    let f = fixture().await;
    let user = UserId::new();
    let store = LocalAssetStore::new(&f.dir.join("assets")).unwrap();

    // What an upload does: the original, its WebP copies and the row that lists them.
    let mut png = Vec::new();
    image::RgbImage::from_pixel(1200, 600, image::Rgb([30, 90, 160]))
      .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
      .unwrap();
    let info = inspect(&png).unwrap();
    let processed = process(&png, &info).unwrap();
    let sha = sha256_hex(&png);
    store.put(&original_key(&sha), png.clone()).await.unwrap();
    let mut variants = Vec::new();
    for v in processed.variants {
      store.put(&variant_key(&sha, v.width), v.bytes.clone()).await.unwrap();
      variants.push(AssetVariant {
        width: v.width,
        height: v.height,
        bytes: v.bytes.len() as u64,
      });
    }
    let (asset, _) = get_or_create_asset(
      &f.db,
      NewAsset {
        sha256: sha,
        mime: info.mime.into(),
        width: Some(info.width),
        height: Some(info.height),
        bytes: png.len() as u64,
        variants,
        placeholder: processed.placeholder,
        uploaded_by: user,
      },
    )
    .await
    .unwrap();

    let page = create_page(&f.db, user, None, "Pictures", Placement::Last).await.unwrap();
    f.content
      .save_draft(
        page.id,
        doc(serde_json::json!({ "type": "doc", "content": [{ "type": "image", "attrs": { "assetId": asset.id.to_string(), "alt": "A blue field" } }] })),
        None,
        user,
      )
      .await
      .unwrap();
    f.publisher.publish(user).await.unwrap();

    let site = f.dir.join("out/site");
    let html = std::fs::read_to_string(site.join("pictures/index.html")).unwrap();
    for expected in [
      format!("/_a/{}-480.webp 480w", asset.id),
      format!("/_a/{}-960.webp 960w", asset.id),
      format!("/_a/{}-1200.webp 1200w", asset.id),
      "sizes=\"(min-width: 800px) 760px, 100vw\"".into(),
      "background-color:#1e5aa0".into(),
    ] {
      assert!(html.contains(&expected), "missing {expected} in {html}");
    }
    for width in [480, 960, 1200] {
      assert!(site.join(format!("_a/{}-{width}.webp", asset.id)).is_file(), "missing the {width}px file");
    }
  }

  #[tokio::test]
  async fn publishing_after_editing_one_page_renders_only_that_page() {
    use yourdocs_store::rename_page;

    let f = fixture().await;
    let user = UserId::new();
    let paragraph = |text: &str| doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }] }));

    let mut drafts = Vec::new();
    for title in ["One", "Two", "Three"] {
      let page = create_page(&f.db, user, None, title, Placement::Last).await.unwrap();
      let draft = f.content.save_draft(page.id, paragraph("first version"), None, user).await.unwrap();
      drafts.push((page, draft));
    }

    // The first publish renders everything: three pages and the 404 page.
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders(), 4);

    // Publishing again with nothing changed renders nothing.
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders(), 4);

    // Editing one page renders that page only, and the site still has every page.
    let (page, draft) = &drafts[1];
    f.content.save_draft(page.id, paragraph("second version"), draft.base_revision_id, user).await.unwrap();
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders(), 5);

    let site = f.dir.join("out/site");
    assert!(std::fs::read_to_string(site.join("two/index.html")).unwrap().contains("second version"));
    assert!(std::fs::read_to_string(site.join("one/index.html")).unwrap().contains("first version"));
    assert!(std::fs::read_to_string(site.join("three/index.html")).unwrap().contains("first version"));

    // A rename changes the navigation of the pages around it, so they render again; the page itself too.
    rename_page(&f.db, drafts[0].0.id, "Uno", yourdocs_core::OldUrls::Redirect).await.unwrap();
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders(), 5 + 4);
    assert!(std::fs::read_to_string(site.join("three/index.html")).unwrap().contains(">Uno<"));
  }

  #[tokio::test]
  async fn each_kind_of_problem_blocks_the_publish_and_names_its_page() {
    use yourdocs_core::ProblemKind;

    let f = fixture().await;
    let user = UserId::new();
    let save = |page: PageId, content: serde_json::Value| {
      let (content_store, doc) = (f.content.clone(), doc(serde_json::json!({ "type": "doc", "content": content })));
      async move {
        let base = content_store.draft(page).await.unwrap().and_then(|d| d.base_revision_id);
        content_store.save_draft(page, doc, base, user).await.unwrap();
      }
    };

    let fine = create_page(&f.db, user, None, "Fine", Placement::Last).await.unwrap();
    let broken = create_page(&f.db, user, None, "Broken links", Placement::Last).await.unwrap();
    let alt = create_page(&f.db, user, None, "No alt", Placement::Last).await.unwrap();
    let props = create_page(&f.db, user, None, "Bad props", Placement::Last).await.unwrap();
    let doomed = create_page(&f.db, user, None, "Doomed", Placement::Last).await.unwrap();
    let cases = [(&broken, ProblemKind::BrokenLink), (&alt, ProblemKind::MissingAlt), (&props, ProblemKind::InvalidProps)];

    let link = |id: PageId| serde_json::json!({ "type": "paragraph", "content": [{ "type": "text", "text": "see", "marks": [{ "type": "link", "attrs": { "pageId": id.to_string() } }] }] });
    save(fine.id, serde_json::json!([{ "type": "paragraph" }])).await;
    save(broken.id, serde_json::json!([link(doomed.id)])).await;
    save(alt.id, serde_json::json!([{ "type": "image", "attrs": { "assetId": "x", "alt": "" } }])).await;
    // `columns` must be "2" or "3".
    save(props.id, serde_json::json!([{ "type": "cards", "attrs": { "columns": "9", "cards": [] } }])).await;
    yourdocs_store::delete_page(&f.db, doomed.id).await.unwrap();

    let blocked = f.publisher.publish(user).await;
    let Err(PublishError::Blocked(problems)) = blocked else {
      panic!("expected the publish to be blocked")
    };

    for (page, kind) in cases {
      assert!(
        problems.iter().any(|p| p.page_title == page.title && p.kind == kind),
        "{} should be flagged as {kind:?}: {problems:?}",
        page.title
      );
    }
    assert!(problems.iter().all(|p| p.page_title != "Fine"));
    // Nothing was published.
    assert!(!f.dir.join("out/site").exists());

    // Fixing every page lets the publish through.
    save(broken.id, serde_json::json!([{ "type": "paragraph" }])).await;
    save(alt.id, serde_json::json!([{ "type": "image", "attrs": { "assetId": "x", "alt": "A chart" } }])).await;
    save(props.id, serde_json::json!([{ "type": "cards", "attrs": { "columns": "3", "cards": [] } }])).await;
    f.publisher.publish(user).await.unwrap();
  }

  #[tokio::test]
  async fn editing_a_snippet_renders_exactly_the_pages_that_use_it() {
    use yourdocs_store::{save_snippet, save_variable};

    let f = fixture().await;
    let user = UserId::new();
    let para = |text: &str| serde_json::json!({ "type": "paragraph", "content": [{ "type": "text", "text": text }] });
    save_variable(&f.db, user, "version", "1.0").await.unwrap();
    save_snippet(&f.db, user, "prereqs", &doc(serde_json::json!({ "type": "doc", "content": [para("Install the CLI first.")] })))
      .await
      .unwrap();

    // Twelve pages use the snippet (and a variable too), three do not.
    let mut titles = Vec::new();
    for (i, uses) in (0..15).map(|i| (i, i < 12)) {
      let title = format!("Page{i}");
      let page = create_page(&f.db, user, None, &title, Placement::Last).await.unwrap();
      let content = if uses {
        serde_json::json!([para("Intro"), { "type": "snippet", "attrs": { "key": "prereqs" } }, { "type": "paragraph", "content": [{ "type": "variable", "attrs": { "key": "version" } }] }])
      } else {
        serde_json::json!([para("Nothing shared")])
      };
      f.content.save_draft(page.id, doc(serde_json::json!({ "type": "doc", "content": content })), None, user).await.unwrap();
      titles.push((title, uses));
    }

    f.publisher.publish(user).await.unwrap();
    let first = f.publisher.renders();
    let site = f.dir.join("out/site");
    let read = |t: &str| std::fs::read_to_string(site.join(t.to_lowercase()).join("index.html")).unwrap();
    assert!(read("Page0").contains("Install the CLI first.") && read("Page0").contains("1.0"));

    // Editing the snippet re-renders the twelve that use it and none of the others.
    save_snippet(
      &f.db,
      user,
      "prereqs",
      &doc(serde_json::json!({ "type": "doc", "content": [para("Install the CLI, version 2, first.")] })),
    )
    .await
    .unwrap();
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders() - first, 12);
    for (title, uses) in &titles {
      assert_eq!(read(title).contains("version 2"), *uses, "{title}");
    }

    // The same for a variable.
    let before = f.publisher.renders();
    save_variable(&f.db, user, "version", "2.0").await.unwrap();
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders() - before, 12);
    assert!(read("Page2").contains("2.0"));

    // An entry that does not exist blocks the publish and names the page.
    let page = create_page(&f.db, user, None, "Lost", Placement::Last).await.unwrap();
    f.content
      .save_draft(
        page.id,
        doc(serde_json::json!({ "type": "doc", "content": [{ "type": "snippet", "attrs": { "key": "nope" } }] })),
        None,
        user,
      )
      .await
      .unwrap();
    let blocked = f.publisher.publish(user).await;
    assert!(
      matches!(&blocked, Err(PublishError::Blocked(p)) if p[0].page_title == "Lost" && p[0].message.contains("snippet nope")),
      "{blocked:?}"
    );
  }

  #[tokio::test]
  async fn a_marked_term_is_a_dfn_linking_to_the_generated_glossary_page() {
    use yourdocs_store::save_term;

    let f = fixture().await;
    let user = UserId::new();
    save_term(&f.db, user, "Blue-green deploy", "Switching all traffic to a new release at once.").await.unwrap();
    save_term(&f.db, user, "API", "A way for programs to talk to each other.").await.unwrap();

    let page = create_page(&f.db, user, None, "Deploying", Placement::Last).await.unwrap();
    f.content
      .save_draft(
        page.id,
        doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
          { "type": "text", "text": "Use a " },
          { "type": "text", "text": "blue-green deploy", "marks": [{ "type": "term", "attrs": { "term": "Blue-green deploy" } }] },
        ] }] })),
        None,
        user,
      )
      .await
      .unwrap();
    f.publisher.publish(user).await.unwrap();

    let site = f.dir.join("out/site");
    let html = std::fs::read_to_string(site.join("deploying/index.html")).unwrap();
    assert!(
      html.contains("<dfn class=\"term\"><a href=\"/glossary/#blue-green-deploy\" data-definition=\"Switching all traffic to a new release at once.\">blue-green deploy</a></dfn>"),
      "{html}"
    );
    // The page only carries the term it uses.
    assert!(!html.contains("talk to each other"));

    // The Glossary page lists every term in alphabetical order, with the anchors the links use.
    let glossary = std::fs::read_to_string(site.join("glossary/index.html")).unwrap();
    let (api, blue) = (glossary.find("id=\"api\"").unwrap(), glossary.find("id=\"blue-green-deploy\"").unwrap());
    assert!(api < blue && glossary.contains("talk to each other"));
  }
}
