use std::{collections::HashMap, sync::Arc};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use yourdocs_core::{AssetStore, Broadcast, ContentStore, Document, Event, Page, PageId, Release, ReleaseStatus, RevisionId, UserId, original_key};
use yourdocs_render::{Pool, Shell, highlight_css};
use yourdocs_store::{DatabaseConnection, add_release_pages, assets_by_ids, create_release, get_setting, list_pages, set_release_status};

use crate::{
  PublishError, generated, search,
  site::{image_asset_ids, nav_tree, output_path, page_urls},
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
}

struct PageSource {
  page: Page,
  doc: Document,
  revision: RevisionId,
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
    }
  }

  pub fn target(&self) -> &PublishTarget {
    &self.target
  }

  /// Publishes every page's latest saved draft as a new release and makes it live.
  pub async fn publish(&self, user: UserId) -> Result<Release, PublishError> {
    let _turn = self.running.lock().await;

    let pages = list_pages(&self.db).await?;
    let sources = self.sources(pages, user).await?;

    let tree: Vec<Value> = sources
      .iter()
      .map(|s| json!({ "id": s.page.id, "parent_id": s.page.parent_id, "slug": s.page.slug, "title": s.page.title }))
      .collect();
    let release = create_release(&self.db, user, Value::Array(tree).to_string(), "[]".into()).await?;

    match self.build_and_deploy(&release, &sources).await {
      Ok(hashes) => {
        add_release_pages(&self.db, release.id, hashes).await?;
        set_release_status(&self.db, release.id, ReleaseStatus::Live).await?;
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

    for page in pages {
      let revision = self.content.ensure_revision(page.id, user).await?;
      sources.push(PageSource {
        page,
        doc: revision.doc,
        revision: revision.id,
      });
    }

    Ok(sources)
  }

  /// Renders everything, then deploys. Nothing reaches the target unless every page rendered.
  async fn build_and_deploy(&self, release: &Release, sources: &[PageSource]) -> Result<Vec<(PageId, RevisionId, String)>, PublishError> {
    let pages: Vec<Page> = sources.iter().map(|s| s.page.clone()).collect();
    let urls = page_urls(&pages);
    let nav = nav_tree(&pages, &urls);
    let site_name = get_setting(&self.db, "site.name").await?.unwrap_or_else(|| "Documentation".into());
    let today = jiff::Zoned::now().date().to_string();

    let (asset_files, asset_meta) = self.asset_files(sources).await?;

    let bundle_files = bundle_files(&self.bundles);
    let stylesheets: Vec<String> = bundle_files.iter().filter(|(name, _)| name.ends_with(".css")).map(|(name, _)| format!("/{name}")).collect();
    let script = bundle_files.iter().find(|(name, _)| name.ends_with(".js")).map(|(name, _)| format!("/{name}")).unwrap_or_default();

    // The jobs own their data: borrowed closures here make the future too lifetime-generic for the HTTP handler that awaits it.
    let jobs: Vec<RenderJob> = sources
      .iter()
      .map(|source| RenderJob {
        pool: self.pool.clone(),
        input: json!({
          "doc": source.doc,
          "context": {
            "site": { "name": site_name },
            "page": { "id": source.page.id, "title": source.page.title },
            "nav": nav,
            "urls": urls,
            "assets": source_assets(&source.doc, &asset_meta),
            "today": today,
          },
        }),
        title: source.page.title.clone(),
        site_name: site_name.clone(),
        stylesheets: stylesheets.clone(),
        script: script.clone(),
        page_id: source.page.id,
        revision: source.revision,
        path: output_path(&urls[&source.page.id]),
      })
      .collect();

    let rendered: Vec<(OutputFile, (PageId, RevisionId, String))> = stream::iter(jobs.into_iter().map(render_job)).buffer_unordered(RENDER_CONCURRENCY).try_collect().await?;

    let (mut files, hashes): (Vec<_>, Vec<_>) = rendered.into_iter().unzip();

    let listed: Vec<(&Page, &str)> = pages.iter().map(|p| (p, urls[&p.id].as_str())).collect();
    let site_url = self.config.site_url.as_str();
    let generated = [
      ("sitemap.xml", generated::sitemap(site_url, &listed)),
      ("robots.txt", generated::robots(site_url)),
      ("llms.txt", generated::llms_txt(&site_name, site_url, &listed)),
      ("_redirects", generated::redirects(listed.first().map(|(_, url)| *url))),
    ];
    files.extend(generated.into_iter().map(|(path, text)| OutputFile {
      path: path.into(),
      bytes: text.into_bytes(),
    }));

    // The page people land on for a URL that does not exist, with the site navigation to find their way.
    let not_found = RenderJob {
      pool: self.pool.clone(),
      input: json!({
        "doc": { "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "There is no page at this address. Use the navigation or the search to find what you were looking for." }] }] },
        "context": { "site": { "name": site_name }, "page": { "title": "Page not found" }, "nav": nav, "urls": urls, "assets": {}, "today": today },
      }),
      title: "Page not found".into(),
      site_name: site_name.clone(),
      stylesheets: stylesheets.clone(),
      script: script.clone(),
      page_id: PageId::new(),
      revision: RevisionId::new(),
      path: "404.html".into(),
    };
    files.push(render_job(not_found).await?.0);

    let html: Vec<&OutputFile> = files.iter().filter(|f| f.path.ends_with(".html")).collect();
    if let Some(index) = search::build_index(&self.config.pagefind, release.id, &html).await? {
      files.extend(index);
    }

    files.extend(asset_files);
    files.extend(bundle_files.into_iter().map(|(path, bytes)| OutputFile { path, bytes }));

    self.target.publish(release.id, files).await?;

    Ok(hashes)
  }

  /// The image files the pages use, and what the renderer needs to know about them.
  async fn asset_files(&self, sources: &[PageSource]) -> Result<(Vec<OutputFile>, HashMap<String, Value>), PublishError> {
    let mut ids: Vec<_> = sources.iter().flat_map(|s| image_asset_ids(&s.doc)).collect();
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
      meta.insert(asset.id.to_string(), json!({ "src": format!("/{path}"), "width": asset.width, "height": asset.height }));
    }

    Ok((files, meta))
  }
}

struct RenderJob {
  pool: Arc<Pool>,
  input: Value,
  title: String,
  site_name: String,
  stylesheets: Vec<String>,
  script: String,
  page_id: PageId,
  revision: RevisionId,
  path: String,
}

async fn render_job(job: RenderJob) -> Result<(OutputFile, (PageId, RevisionId, String)), PublishError> {
  let body = job.pool.render(job.input.clone()).await.map_err(|source| PublishError::Render { page: job.title.clone(), source })?;
  let html = yourdocs_render::document(&Shell {
    title: &job.title,
    site_name: &job.site_name,
    body: &body,
    input: &job.input.to_string(),
    stylesheets: &job.stylesheets,
    script: &job.script,
  });

  let hash = Sha256::digest(html.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();

  Ok((
    OutputFile {
      path: job.path,
      bytes: html.into_bytes(),
    },
    (job.page_id, job.revision, hash),
  ))
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

    assert!(matches!(f.publisher.publish(user).await, Err(PublishError::Render { .. })));
    assert_eq!(std::fs::read_to_string(f.dir.join("out/site/page/index.html")).unwrap(), before);
    // No half-written release is left behind either.
    assert_eq!(std::fs::read_dir(f.dir.join("out/releases")).unwrap().count(), 1);
  }
}
