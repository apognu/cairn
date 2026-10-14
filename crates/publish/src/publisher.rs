use std::{
  collections::{HashMap, HashSet},
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use cairn_core::{
  ApiConfig, AssetId, AssetStore, Broadcast, ContentStore, Document, Event, Library, Page, PageId, PageKind, Release, ReleaseId, ReleaseStatus, RevisionId, Settings, UserId, effective_redirects,
  expand, extract_links, header_section, original_key, page_urls, term_anchors, term_refs, variant_key,
};
use cairn_render::{Pool, Shell, highlight_css};
use cairn_store::{
  DatabaseConnection, ReleasePage, ReleaseSpec, Staging, add_release_pages, assets_by_ids, create_release, get_release, get_setting, kept_render_keys, latest_live_release, list_api_configs,
  live_release, load_library, load_settings, make_release_live, record_release_specs, release_specs, set_release_deployment, set_release_render_keys, set_release_status, set_setting, staging,
};
use futures_util::{StreamExt, TryStreamExt, stream};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
  ApiProblem, PublishError,
  api::{ApiHosts, ResolvedApi, ResolvedSpec, SourceFetcher, resolve},
  content::content_hash,
  generated,
  generated::Listed,
  glossary::glossary_doc,
  markdown::{Site, page_markdown},
  search,
  site::{breadcrumbs, header_items, image_asset_ids, markdown_path, nav_tree, output_path, previous_next, section_back},
  target::{Deployed, OutputFile, Progress, PublishTarget},
};

/// The browser-side files every page links to; embedded in the binary and written into each release.
pub struct Bundles {
  pub client_js: Vec<u8>,
  pub site_css: Vec<u8>,
  /// The self-hosted API reference script; only pages of that kind load it.
  pub scalar_js: Vec<u8>,
  /// Identifies this set of bundles, `ssr.js` included: pages rendered by other bundles are rendered again.
  pub hash: String,
}

impl Bundles {
  pub fn new(ssr_js: &str, client_js: Vec<u8>, site_css: Vec<u8>, scalar_js: Vec<u8>) -> Self {
    let mut digest = Sha256::new();
    for part in [ssr_js.as_bytes(), &client_js, &site_css, &scalar_js] {
      digest.update((part.len() as u64).to_le_bytes());
      digest.update(part);
    }

    Self {
      client_js,
      site_css,
      scalar_js,
      hash: digest.finalize().iter().map(|b| format!("{b:02x}")).collect(),
    }
  }
}

/// Settings that come from the deployment rather than the content.
pub struct PublishConfig {
  /// The public address of the site without a trailing slash, for the sitemap and llms.txt.
  pub site_url: String,
  /// The Pagefind executable (a path or a name on `PATH`).
  pub pagefind: String,
  /// Where API sources on GitHub are read from; only tests and mirrors change it.
  pub api_hosts: ApiHosts,
}

/// How many pages are in V8 at once. Below the pool's queue size, so a big site never gets "busy" from its own publish.
const RENDER_CONCURRENCY: usize = 16;

/// How many API pages fetch their specs at once.
const API_CONCURRENCY: usize = 4;

/// The setting that remembers which bundles the live site was last rendered with.
const BUNDLES_SETTING: &str = "publish.bundles";

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
  fetcher: SourceFetcher,
  /// How many pages went through V8 since startup; pages whose output could be reused are not counted.
  renders: Arc<AtomicUsize>,
}

/// What a release is built from: the tree, its redirects, the pages to render, and the settings and library they are rendered with.
struct Contents<'a> {
  tree_pages: &'a [Page],
  rules: &'a [(String, String)],
  sources: &'a [PageSource],
  settings: &'a Settings,
  library: &'a Library,
}

struct PageSource {
  page: Page,
  doc: Document,
  revision: RevisionId,
  /// When the revision being published was saved: the page's "last updated".
  updated: jiff::Timestamp,
  /// An API page has no document: it is its merged spec.
  api: Option<ResolvedApi>,
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
      broadcast: parts.broadcast,
      running: tokio::sync::Mutex::new(()),
      fetcher: SourceFetcher::new(parts.config.api_hosts.clone()).expect("the HTTP client for API sources must build"),
      config: parts.config,
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

  /// Tells every subscriber about something that happened outside publishing (a page lock changing hands).
  pub fn notify(&self, event: Event) {
    self.broadcast.send(event);
  }

  /// Events sent from now on: releases going live, the progress of the build being sent, and whatever `notify` was given.
  pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
    self.broadcast.subscribe()
  }

  /// The public address of the site, without a trailing slash.
  pub fn site_url(&self) -> &str {
    &self.config.site_url
  }

  /// Publishes every page's latest saved draft as a new release and makes it live.
  pub async fn publish(&self, user: UserId) -> Result<Release, PublishError> {
    self.build(user, true).await
  }

  /// Builds a release from every page's latest saved draft and hands it to the target without making it live: it can be looked at, and
  /// then promoted.
  pub async fn preview(&self, user: UserId) -> Result<Release, PublishError> {
    if !self.target.capabilities().preview {
      return Err(PublishError::Core(cairn_core::CoreError::Invalid("this publish target has no previews".into())));
    }

    self.build(user, false).await
  }

  /// Makes a preview the live site, exactly as it was built. A preview older than the live site is refused: it would put back what
  /// was published since, and a new preview takes a moment.
  pub async fn promote(&self, id: ReleaseId) -> Result<Release, PublishError> {
    if !self.target.capabilities().promote {
      return Err(PublishError::Core(cairn_core::CoreError::Invalid(
        "this publish target cannot make a preview the live site; publish to deploy it".into(),
      )));
    }
    let _turn = self.running.lock().await;

    let (release, version) = get_release(&self.db, id).await?;
    if release.status != ReleaseStatus::Preview {
      return Err(PublishError::Core(cairn_core::CoreError::Invalid("only a preview can be promoted".into())));
    }
    if latest_live_release(&self.db).await?.is_some_and(|live| live.id > release.id) {
      return Err(PublishError::Stale("the site was published after this preview was made; make a new preview".into()));
    }

    self.target.promote(release.id, version.as_deref()).await?;
    self.make_live(release.id).await?;
    tracing::info!(release_id = %release.id, "preview promoted");
    self.broadcast.send(Event::Published { release: release.id });

    Ok(Release {
      status: ReleaseStatus::Live,
      preview_url: None,
      ..release
    })
  }

  async fn make_live(&self, id: ReleaseId) -> Result<(), PublishError> {
    // The site is already live: a failure here leaves the record behind it, which the log has to say.
    if let Err(error) = make_release_live(&self.db, id).await {
      tracing::error!(release_id = %id, %error, "the release is live but could not be marked so");
      return Err(error.into());
    }
    // Every live release is rendered by these bundles; the next start republishes only when they change. Failing to note it only costs
    // a needless republish, so it does not turn a live release into an error.
    if let Err(error) = set_setting(&self.db, BUNDLES_SETTING, &self.bundles.hash).await {
      tracing::warn!(release_id = %id, %error, "could not record the bundles of the live release");
    }
    // Leftovers only cost disk space, so a failure here is not the publish's.
    if let Err(error) = self.prune_renders().await {
      tracing::warn!(release_id = %id, %error, "could not delete unused render cache entries");
    }

    Ok(())
  }

  /// Deletes the cached renders that neither the live release nor a preview made after it uses. Called with the build lock held, so no
  /// build is adding entries meanwhile.
  async fn prune_renders(&self) -> Result<(), PublishError> {
    let Some(kept) = kept_render_keys(&self.db).await? else {
      return Ok(());
    };

    let mut pruned = 0;
    for key in self.assets.list(RENDERS_FOLDER).await? {
      if !kept.contains(&key) {
        self.assets.delete(&key).await?;
        pruned += 1;
      }
    }
    if pruned > 0 {
      tracing::info!(pruned, "unused render cache entries deleted");
    }

    Ok(())
  }

  async fn build(&self, user: UserId, live: bool) -> Result<Release, PublishError> {
    let _turn = self.running.lock().await;

    // One snapshot for the checks and the build: a save or a tree change made meanwhile waits for the next publish instead of going out
    // unchecked.
    let staging = staging(&self.db).await?;
    let tree_pages = &staging.pages;
    // Every build reads the specs again; one that cannot be read stops the publish before anything is created.
    let apis = self.resolve_apis(tree_pages, &staging.api_configs).await?;

    // Nothing is created, rendered or written until the site passes its checks, links to operations included: those are compared with
    // the specs of this very build.
    let anchors: crate::OperationAnchors = apis.iter().map(|api| (api.page_id, api.operations.iter().map(cairn_core::Operation::anchor).collect())).collect();
    let problems = crate::changes::check_with(&staging, &self.pool, &anchors).await?;
    if !problems.is_empty() {
      return Err(PublishError::Blocked(problems));
    }

    // Links are navigation only: nothing to render, nothing in the sitemap.
    let mut sources = self.sources(&staging, user).await?;
    sources.extend(api_sources(tree_pages, apis));

    let rules = effective_redirects(&staging.redirects, tree_pages);
    let settings = &staging.settings;
    // Frozen with the release: the next "what changed" comparison reads them back, and a re-render uses them.
    let release = create_release(
      &self.db,
      user,
      serde_json::to_string(tree_pages).map_err(anyhow::Error::from)?,
      serde_json::to_string(&rules).map_err(anyhow::Error::from)?,
      settings,
    )
    .await?;

    let contents = Contents {
      tree_pages,
      rules: &rules,
      sources: &sources,
      settings,
      library: &staging.library,
    };
    self.deploy(release, &contents, live).await
  }

  /// Renders the live release again when the bundles changed since it was last rendered: the same pages at the same revisions, the same
  /// specs and the same settings, with the new components. Nothing unpublished goes out. A failure leaves the live site as it is, and the
  /// next start tries again.
  pub async fn republish_for_bundles(&self) -> Result<Option<Release>, PublishError> {
    let _turn = self.running.lock().await;
    if get_setting(&self.db, BUNDLES_SETTING).await?.as_deref() == Some(self.bundles.hash.as_str()) {
      return Ok(None);
    }

    let (Some(live), Some(previous)) = (live_release(&self.db).await?, latest_live_release(&self.db).await?) else {
      // Nothing published yet: the first publish renders with these bundles anyway.
      set_setting(&self.db, BUNDLES_SETTING, &self.bundles.hash).await?;
      return Ok(None);
    };

    let library = load_library(&self.db).await?;
    let mut sources = Vec::new();
    for page in live.pages.iter().filter(|p| p.kind == PageKind::Page) {
      let Some(revision) = live.revisions.get(&page.id) else { continue };
      let revision = self.content.revision(*revision).await?;
      sources.push(PageSource {
        page: page.clone(),
        doc: expand(&revision.doc, &library).0,
        revision: revision.id,
        updated: revision.created_at,
        api: None,
      });
    }

    // The specs and settings as they were: a new bundle must not pull in what upstream changed since, nor what was saved and not published.
    // A release made before settings were recorded has only the current ones to go by.
    let recorded = release_specs(&self.db, previous.id).await?;
    let current = list_api_configs(&self.db).await?;
    let mut apis = Vec::new();
    for spec in recorded {
      let bytes = self.assets.get(&ResolvedApi::store_key(&spec.spec_hash)).await?;
      let config = spec.config.unwrap_or_else(|| current.get(&spec.page_id).cloned().unwrap_or_default());
      apis.push(ResolvedApi {
        page_id: spec.page_id,
        bytes,
        hash: spec.spec_hash,
        sources: spec.sources,
        operations: spec.operations,
        config,
      });
    }
    sources.extend(api_sources(&live.pages, apis));
    let settings = match live.settings {
      Some(settings) => settings,
      None => load_settings(&self.db).await?,
    };

    let release = create_release(
      &self.db,
      previous.created_by,
      serde_json::to_string(&live.pages).map_err(anyhow::Error::from)?,
      serde_json::to_string(&live.redirects).map_err(anyhow::Error::from)?,
      &settings,
    )
    .await?;
    let contents = Contents {
      tree_pages: &live.pages,
      rules: &live.redirects,
      sources: &sources,
      settings: &settings,
      library: &library,
    };
    let release = self.deploy(release, &contents, true).await?;
    tracing::info!(release_id = %release.id, pages = release.page_count, "republished for new bundles");

    Ok(Some(release))
  }

  /// Renders and hands over a release that was just created, then marks it live, as a preview, or failed.
  async fn deploy(&self, release: Release, contents: &Contents<'_>, live: bool) -> Result<Release, PublishError> {
    let sources = contents.sources;
    match self.build_and_deploy(&release, contents, live).await {
      Ok(deployed) => {
        let status = if live { ReleaseStatus::Live } else { ReleaseStatus::Preview };
        // A live release stays `building` until `make_live` swaps it for the previous live one in one step.
        let recorded = if live { ReleaseStatus::Building } else { ReleaseStatus::Preview };
        set_release_deployment(&self.db, release.id, recorded, deployed.version, deployed.preview_url.clone()).await?;
        if live {
          self.make_live(release.id).await?;
          self.broadcast.send(Event::Published { release: release.id });
        }
        tracing::info!(release_id = %release.id, pages = sources.len(), live, "release is built");

        Ok(Release {
          status,
          page_count: sources.len() as u32,
          preview_url: deployed.preview_url,
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

  /// The pages of the snapshot at the revisions it saw. A page nobody has edited gets an empty revision.
  async fn sources(&self, staging: &Staging, user: UserId) -> Result<Vec<PageSource>, PublishError> {
    let pages = staging.pages.iter().filter(|p| p.kind == PageKind::Page);
    let mut sources = Vec::with_capacity(staging.pages.len());

    for page in pages {
      let revision = match staging.revisions.get(&page.id) {
        Some(id) => self.content.revision(*id).await?,
        None => self.content.ensure_revision(page.id, user).await?,
      };
      sources.push(PageSource {
        page: page.clone(),
        // Variables and snippets are written into the page here, so everything after (the render, the Markdown, the cache key)
        // sees the page as readers do, and a changed entry changes exactly the pages that use it.
        doc: expand(&revision.doc, &staging.library).0,
        revision: revision.id,
        updated: revision.created_at,
        api: None,
      });
    }

    Ok(sources)
  }

  /// Fetches and merges the sources of an API page's settings without publishing anything, for "Check sources now".
  pub async fn check_api(&self, config: &ApiConfig) -> Result<ResolvedSpec, String> {
    resolve(&self.fetcher, config).await
  }

  /// Fetches and merges the specs of every API page, keeping the merged documents in the asset store for rollbacks and republishes.
  /// Every page that fails is named.
  async fn resolve_apis(&self, tree_pages: &[Page], configs: &HashMap<PageId, ApiConfig>) -> Result<Vec<ResolvedApi>, PublishError> {
    let jobs: Vec<(Page, ApiConfig)> = tree_pages
      .iter()
      .filter(|p| p.kind == PageKind::Api)
      .map(|page| (page.clone(), configs.get(&page.id).cloned().unwrap_or_default()))
      .collect();

    // A named function, not a closure: borrowed closures make this future too lifetime-generic for the HTTP handler that awaits it.
    let results: Vec<_> = stream::iter(jobs.into_iter().map(|(page, config)| resolve_page(&self.fetcher, page, config)))
      .buffer_unordered(API_CONCURRENCY)
      .collect()
      .await;

    let mut apis = Vec::new();
    let mut problems = Vec::new();
    for (page, result, config) in results {
      match result {
        Ok(spec) => {
          self.assets.put(&ResolvedApi::store_key(&spec.hash), spec.bytes.clone()).await?;
          apis.push(ResolvedApi {
            page_id: page.id,
            bytes: spec.bytes,
            hash: spec.hash,
            sources: spec.sources,
            operations: spec.operations,
            config,
          });
        }
        Err(message) => problems.push(ApiProblem {
          page_title: page.title.clone(),
          message,
        }),
      }
    }

    if !problems.is_empty() {
      problems.sort_by(|a, b| a.page_title.cmp(&b.page_title));
      return Err(PublishError::Api(problems));
    }

    Ok(apis)
  }

  /// Renders everything, then deploys. Nothing reaches the target unless every page rendered.
  async fn build_and_deploy(&self, release: &Release, contents: &Contents<'_>, live: bool) -> Result<Deployed, PublishError> {
    let &Contents {
      tree_pages,
      rules,
      sources,
      settings,
      library,
    } = contents;
    let pages: Vec<Page> = sources.iter().map(|s| s.page.clone()).collect();
    let urls = page_urls(tree_pages);
    let header = header_items(tree_pages, &urls);
    // What link cards and other components show about the pages they point to; a rename or new description reaches them at the next publish.
    let url_values: serde_json::Map<String, Value> = urls.iter().map(|(id, url)| (id.to_string(), Value::String(url.clone()))).collect();
    let page_info = tree_pages
      .iter()
      .filter(|p| p.kind.has_url())
      .map(|p| (p.id.to_string(), json!({ "title": p.title, "description": p.description })))
      .collect::<serde_json::Map<String, Value>>();
    // The glossary: every term in order, where its entry is, and where the Glossary page itself lives (never on top of a real page).
    let mut glossary_terms: Vec<String> = library.terms.keys().cloned().collect();
    glossary_terms.sort_by_key(|t| t.to_lowercase());
    let anchors = term_anchors(&glossary_terms);
    let glossary_url = if urls.values().any(|u| u == "/glossary/") { "/glossary-terms/" } else { "/glossary/" };
    let site_name = settings.site_name.clone();
    let today = jiff::Zoned::now().date().to_string();

    let (asset_files, asset_meta) = self.asset_files(sources, [settings.logo_asset_id, settings.logo_dark_asset_id].into_iter().flatten()).await?;
    let logo = settings.logo_asset_id.and_then(|id| asset_meta.get(&id.to_string()).cloned());
    let logo_dark = settings.logo_dark_asset_id.and_then(|id| asset_meta.get(&id.to_string()).cloned());
    let site_ctx = json!({ "name": site_name, "logo": logo, "logoDark": logo_dark, "headerLinks": settings.header_links, "footer": settings.footer });
    let theme_css = settings.theme_css();

    let mut bundle_files = bundle_files(&self.bundles);
    // Only API pages load it, so a site without any does not carry its 4 MB in every release.
    let scalar_url = sources.iter().any(|s| s.api.is_some()).then(|| {
      let (name, bytes) = scalar_file(&self.bundles);
      bundle_files.push((name.clone(), bytes));
      format!("/{name}")
    });
    let stylesheets: Vec<String> = bundle_files.iter().filter(|(name, _)| name.ends_with(".css")).map(|(name, _)| format!("/{name}")).collect();
    let script = bundle_files.iter().find(|(name, _)| name.ends_with(".js")).map(|(name, _)| format!("/{name}")).unwrap_or_default();

    let betas: HashSet<PageId> = sources.iter().filter_map(|s| s.api.as_ref()).filter(|api| api.config.display.beta).map(|api| api.page_id).collect();

    // The jobs own their data: borrowed closures here make the future too lifetime-generic for the HTTP handler that awaits it.
    let jobs: Vec<RenderJob> = sources
      .iter()
      .map(|source| {
        let nav = nav_tree(tree_pages, &urls, &betas, Some(source.page.id));
        let (previous, next) = previous_next(&nav, source.page.id);
        // Only what the page refers to goes into its context: renaming one page must not change the input of every other, or nothing
        // could be reused.
        let referenced: HashSet<String> = extract_links(&source.doc).into_iter().map(|l| l.target.to_string()).collect();
        // The date only matters to components that count down to one, and it changes every day.
        // Only the terms this page marks go into its context, so defining another term leaves it alone.
        let terms: serde_json::Map<String, Value> = term_refs(&source.doc)
          .into_iter()
          .filter_map(|t| {
            Some((
              t.clone(),
              json!({ "definition": library.terms.get(&t)?.definition.plain_text(), "href": format!("{glossary_url}#{}", anchors.get(&t)?) }),
            ))
          })
          .collect();
        let today = if serde_json::to_string(&source.doc.0).is_ok_and(|json| json.contains("\"newUntil\"")) {
          today.as_str()
        } else {
          ""
        };

        let page_url = &urls[&source.page.id];
        let mut input = json!({
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
            "sectionBack": section_back(tree_pages, &urls, source.page.id, &site_name),
            "urls": only(&url_values, &referenced),
            "pages": only(&page_info, &referenced),
            "assets": source_assets(&source.doc, &asset_meta),
            "terms": terms,
            "today": today,
          },
        });
        if let Some(api) = &source.api {
          input["context"]["api"] = api_context(api, page_url, scalar_url.as_deref());
          input["context"]["updated"] = json!("");
          // Only Scalar's own view is offered, no Markdown copy.
          input["context"]["markdown"] = Value::Null;
        }

        RenderJob {
          pool: self.pool.clone(),
          input,
          title: source.page.title.clone(),
          description: source.page.description.clone(),
          site_name: site_name.clone(),
          stylesheets: stylesheets.clone(),
          script: script.clone(),
          theme_css: theme_css.clone(),
          bundle_hash: self.bundles.hash.clone(),
          page_id: source.page.id,
          revision: source.revision,
          path: output_path(&urls[&source.page.id]),
        }
      })
      .collect();

    let rendered: Vec<Rendered> = stream::iter(jobs.into_iter().map(|job| render_job(job, self.assets.clone(), self.renders.clone())))
      .buffer_unordered(RENDER_CONCURRENCY)
      .try_collect()
      .await?;

    let mut render_keys: Vec<String> = rendered.iter().map(|r| r.cache_key.clone()).collect();
    let (mut files, mut hashes): (Vec<_>, Vec<_>) = rendered.into_iter().map(|r| (r.file, (r.page_id, r.revision, r.output_hash))).unzip();
    // An API page has no revision to record; what it was built from goes in `release_api_specs`.
    let apis: Vec<&ResolvedApi> = sources.iter().filter_map(|s| s.api.as_ref()).collect();
    hashes.retain(|(id, _, _)| !apis.iter().any(|a| a.page_id == *id));
    files.extend(apis.iter().map(|api| OutputFile {
      path: api.spec_path(&urls[&api.page_id]),
      bytes: api.bytes.clone(),
    }));

    let glossary = (!glossary_terms.is_empty()).then(|| glossary_doc(&glossary_terms, library, &urls));
    let mut listed: Vec<Listed> = pages
      .iter()
      .map(|p| Listed {
        title: &p.title,
        description: p.description.as_deref(),
        url: &urls[&p.id],
        updated: p.updated_at,
      })
      .collect();
    if glossary.is_some() {
      listed.push(Listed {
        title: "Glossary",
        description: None,
        url: glossary_url,
        updated: library.terms.values().map(|t| t.updated_at).max().unwrap_or_default(),
      });
    }
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

    if let Some(glossary) = glossary {
      files.push(OutputFile {
        path: markdown_path(glossary_url),
        bytes: page_markdown("Glossary", None, &glossary, &site).into_bytes(),
      });

      let referenced: HashSet<String> = extract_links(&glossary).into_iter().map(|l| l.target.to_string()).collect();
      let glossary = RenderJob {
        pool: self.pool.clone(),
        input: json!({
          "doc": glossary,
          // A fixed ID keeps the page searchable and its render cacheable.
          "context": {
            "site": site_ctx,
            "page": { "id": "00000000-0000-7000-8000-00000000610c", "title": "Glossary" },
            "nav": nav_tree(tree_pages, &urls, &betas, None),
            "header": header,
            "urls": only(&url_values, &referenced),
            "pages": only(&page_info, &referenced),
            "assets": {},
            "today": "",
          },
        }),
        title: "Glossary".into(),
        description: None,
        site_name: site_name.clone(),
        stylesheets: stylesheets.clone(),
        script: script.clone(),
        theme_css: theme_css.clone(),
        bundle_hash: self.bundles.hash.clone(),
        page_id: PageId::new(),
        revision: RevisionId::new(),
        path: output_path(glossary_url),
      };
      let rendered = render_job(glossary, self.assets.clone(), self.renders.clone()).await?;
      render_keys.push(rendered.cache_key);
      files.push(rendered.file);
    }

    // The page people land on for a URL that does not exist, with the site navigation to find their way.
    let not_found = RenderJob {
      pool: self.pool.clone(),
      input: json!({
        "doc": { "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "There is no page at this address. Use the navigation or the search to find what you were looking for." }] }] },
        "context": { "site": site_ctx, "page": { "title": "Page not found" }, "notFound": true, "nav": nav_tree(tree_pages, &urls, &betas, None), "header": header, "urls": {}, "assets": {}, "today": "" },
      }),
      title: "Page not found".into(),
      description: None,
      site_name: site_name.clone(),
      stylesheets: stylesheets.clone(),
      script: script.clone(),
      theme_css: theme_css.clone(),
      bundle_hash: self.bundles.hash.clone(),
      page_id: PageId::new(),
      revision: RevisionId::new(),
      path: "404.html".into(),
    };
    let rendered = render_job(not_found, self.assets.clone(), self.renders.clone()).await?;
    render_keys.push(rendered.cache_key);
    files.push(rendered.file);

    let html: Vec<&OutputFile> = files.iter().filter(|f| f.path.ends_with(".html")).collect();
    if let Some(index) = search::build_index(&self.config.pagefind, release.id, &html).await? {
      files.extend(index);
    }

    files.extend(asset_files);
    files.extend(bundle_files.into_iter().map(|(path, bytes)| OutputFile { path, bytes }));

    // Before anything is recorded or sent: a target that would refuse the site says so while nothing has happened.
    let refused = self.target.check(&files);
    if !refused.is_empty() {
      return Err(PublishError::Rejected(refused));
    }

    // Recorded before the switch: once the site is live, what it holds must already be known.
    let release_pages = hashes
      .into_iter()
      .filter_map(|(page_id, revision_id, output_hash)| {
        let source = sources.iter().find(|s| s.page.id == page_id)?;
        Some(ReleasePage {
          page_id,
          revision_id,
          output_hash,
          content_hash: content_hash(&source.doc, tree_pages, &urls, library),
        })
      })
      .collect();
    add_release_pages(&self.db, release.id, release_pages).await?;
    set_release_render_keys(&self.db, release.id, &render_keys).await?;
    let specs: Vec<ReleaseSpec> = apis
      .iter()
      .map(|api| ReleaseSpec {
        page_id: api.page_id,
        spec_hash: api.hash.clone(),
        sources: api.sources.clone(),
        operations: api.operations.clone(),
        config: Some(api.config.clone()),
      })
      .collect();
    record_release_specs(&self.db, release.id, &specs).await?;

    self.target.publish(release.id, files, live, self.progress()).await
  }

  /// Sends what a target reports of its upload to the editors watching. A message per file would flood the channel on a large site, and
  /// subscribers that fall behind lose messages: one per percent is enough for a bar.
  fn progress(&self) -> Arc<Progress> {
    let broadcast = self.broadcast.clone();
    let last = AtomicUsize::new(usize::MAX);

    Progress::new(move |sent, total| {
      let percent = (sent * 100).checked_div(total).unwrap_or(100);
      if last.swap(percent, Ordering::Relaxed) != percent {
        broadcast.send(Event::PublishProgress {
          sent: sent as u64,
          total: total as u64,
        });
      }
    })
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
  bundle_hash: String,
  page_id: PageId,
  revision: RevisionId,
  path: String,
}

/// Where cached renders live in the asset store.
const RENDERS_FOLDER: &str = "renders";

/// Part of every render cache key. The JS bundles are in the key by their hashed names, but the page shell and code highlighting are Rust:
/// a release changes the version, and the number is bumped when either changes between releases. Otherwise unchanged pages would keep
/// the old output for good.
const RENDER_FORMAT: &str = concat!(env!("CARGO_PKG_VERSION"), "/1");

/// A rendered page, the hash of its HTML, and the render cache entry it came from.
struct Rendered {
  file: OutputFile,
  page_id: PageId,
  revision: RevisionId,
  output_hash: String,
  cache_key: String,
}

/// Renders one page, or reuses what an identical render produced before. The cache is content addressed: the key is a hash of everything the
/// HTML depends on (the render input, the page's own title and description, the site name and theme, the stylesheets and script), so the same
/// key always means the same file and nothing ever has to be invalidated; entries no kept release uses are deleted when a release goes live.
async fn render_job<A: AssetStore>(job: RenderJob, cache: Arc<A>, renders: Arc<AtomicUsize>) -> Result<Rendered, PublishError> {
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
      &job.bundle_hash,
    ] {
      digest.update(part.as_bytes());
      digest.update([0]);
    }
    format!("{RENDERS_FOLDER}/{}.html", digest.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>())
  };

  let bytes = if cache.exists(&key).await? {
    cache.get(&key).await?
  } else {
    let body = job.pool.render(job.input.clone()).await.map_err(|source| PublishError::Render { page: job.title.clone(), source })?;
    renders.fetch_add(1, Ordering::Relaxed);
    let html = cairn_render::document(&Shell {
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

  let output_hash = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();

  Ok(Rendered {
    file: OutputFile { path: job.path, bytes },
    page_id: job.page_id,
    revision: job.revision,
    output_hash,
    cache_key: key,
  })
}

async fn resolve_page(fetcher: &SourceFetcher, page: Page, config: ApiConfig) -> (Page, Result<ResolvedSpec, String>, ApiConfig) {
  let result = resolve(fetcher, &config).await;

  (page, result, config)
}

/// The API pages as sources to render: no document, and an id for a revision that is never recorded.
fn api_sources(tree_pages: &[Page], apis: Vec<ResolvedApi>) -> Vec<PageSource> {
  apis
    .into_iter()
    .filter_map(|api| {
      let page = tree_pages.iter().find(|p| p.id == api.page_id)?.clone();

      Some(PageSource {
        page,
        doc: Document::empty(),
        revision: RevisionId::new(),
        updated: jiff::Timestamp::UNIX_EPOCH,
        api: Some(api),
      })
    })
    .collect()
}

/// What an API page's render depends on besides the page: where its spec is, and the operations for the fallback.
fn api_context(api: &ResolvedApi, page_url: &str, scalar_url: Option<&str>) -> Value {
  json!({
    "specUrl": format!("/{}", api.spec_path(page_url)),
    "scalarUrl": scalar_url,
    "operations": api.operations,
    "display": { "servers": api.config.display.servers, "hideModels": api.config.display.hide_models, "requestClient": api.config.display.request_client },
  })
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

fn scalar_file(bundles: &Bundles) -> (String, Vec<u8>) {
  let digest: String = Sha256::digest(&bundles.scalar_js).iter().take(4).map(|b| format!("{b:02x}")).collect();

  (format!("assets/scalar-{digest}.js"), bundles.scalar_js.clone())
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

  use cairn_assets::LocalAssetStore;
  use cairn_core::{LocalBroadcast, Placement, UserId};
  use cairn_store::{SqlContentStore, create_page};

  use super::*;
  use crate::FolderTarget;

  struct Fixture {
    publisher: Publisher<SqlContentStore, LocalAssetStore, LocalBroadcast>,
    db: DatabaseConnection,
    content: SqlContentStore,
    dir: std::path::PathBuf,
  }

  async fn fixture() -> Fixture {
    fixture_with(ApiHosts::default()).await
  }

  async fn fixture_with(api_hosts: ApiHosts) -> Fixture {
    fixture_for(api_hosts, false).await
  }

  /// A publisher whose target is the self-hosted one, which can hold builds that are not live.
  async fn self_hosted_fixture() -> Fixture {
    fixture_for(ApiHosts::default(), true).await
  }

  async fn fixture_for(api_hosts: ApiHosts, self_hosted: bool) -> Fixture {
    let dir = std::env::temp_dir().join(format!("cairn-publish-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = cairn_store::connect_for_tests(&dir).await;

    let script = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/renderer/dist/ssr.js")).expect("run `bun run build` first");
    let pool = Arc::new(Pool::new(&script, 1, 32, Duration::from_secs(5)).unwrap());
    let content = SqlContentStore::new(db.clone());

    let publisher = Publisher::new(PublisherParts {
      db: db.clone(),
      content: content.clone(),
      assets: Arc::new(LocalAssetStore::new(&dir.join("assets")).unwrap()),
      pool,
      target: if self_hosted {
        PublishTarget::SelfHosted(FolderTarget::new(dir.join("out")))
      } else {
        PublishTarget::Folder(FolderTarget::new(dir.join("out")))
      },
      bundles: Bundles::new("ssr", b"console.log(1)".to_vec(), b"body{}".to_vec(), b"window.Scalar = {}".to_vec()),
      config: PublishConfig {
        site_url: "https://docs.example.com".into(),
        pagefind: std::env::var("PAGEFIND_BIN").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../node_modules/.bin/pagefind").into()),
        api_hosts,
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
  async fn a_preview_is_built_without_going_live_and_promoting_it_switches_the_site() {
    let f = self_hosted_fixture().await;
    let user = UserId::new();
    create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();

    let preview = f.publisher.preview(user).await.unwrap();

    assert_eq!(preview.status, ReleaseStatus::Preview);
    assert!(f.dir.join("out/releases").join(preview.id.to_string()).join("guide/index.html").is_file(), "the build is on disk");
    assert!(!f.dir.join("out/site").exists(), "but the site has not switched");
    assert!(latest_live_release(&f.db).await.unwrap().is_none());

    let live = f.publisher.promote(preview.id).await.unwrap();

    assert_eq!(live.status, ReleaseStatus::Live);
    assert_eq!(latest_live_release(&f.db).await.unwrap().unwrap().id, preview.id);
    assert!(f.dir.join("out/site/guide/index.html").is_file(), "the site is the build that was looked at");
    // Once live, it is not a preview any more.
    assert!(f.publisher.promote(preview.id).await.is_err());
  }

  #[tokio::test]
  async fn a_preview_older_than_the_live_site_cannot_be_promoted() {
    let f = self_hosted_fixture().await;
    let user = UserId::new();
    create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();

    let old = f.publisher.preview(user).await.unwrap();
    let published = f.publisher.publish(user).await.unwrap();
    let error = f.publisher.promote(old.id).await.expect_err("the site moved on");

    assert!(matches!(error, PublishError::Stale(_)), "{error}");
    // The newer release is the only live one.
    let (first, _) = cairn_store::get_release(&f.db, old.id).await.unwrap();
    let (second, _) = cairn_store::get_release(&f.db, published.id).await.unwrap();
    assert_eq!((first.status, second.status), (ReleaseStatus::Preview, ReleaseStatus::Live));
  }

  #[tokio::test]
  async fn a_target_without_previews_refuses_one() {
    let f = fixture().await;

    assert!(f.publisher.preview(UserId::new()).await.is_err());
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
  async fn the_upload_is_reported_until_every_file_is_sent() {
    let f = fixture().await;
    let user = UserId::new();
    create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();
    let mut events = f.publisher.subscribe();

    f.publisher.publish(user).await.unwrap();

    let mut reports = Vec::new();
    while let Ok(event) = events.try_recv() {
      if let Event::PublishProgress { sent, total } = event {
        reports.push((sent, total));
      }
    }
    let (first, last) = (reports.first().unwrap(), reports.last().unwrap());
    assert_eq!(first.0, 0, "the first report comes before anything is sent");
    assert!(last.1 > 0 && last.0 == last.1, "the last one says everything went: {reports:?}");
    assert!(reports.windows(2).all(|w| w[0].0 < w[1].0), "each report is news: {reports:?}");
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
    use cairn_core::OldUrls;
    use cairn_store::move_page;

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
    use cairn_core::OldUrls;
    use cairn_store::move_page;

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
    use cairn_store::set_in_header;

    let f = fixture().await;
    let user = UserId::new();
    let guides = create_page(&f.db, user, None, "Guides", Placement::Last).await.unwrap();
    create_page(&f.db, user, Some(guides.id), "Install", Placement::Last).await.unwrap();
    create_page(&f.db, user, None, "About", Placement::Last).await.unwrap();
    set_in_header(&f.db, guides.id, true).await.unwrap();
    f.publisher.publish(user).await.unwrap();

    let read = |path: &str| std::fs::read_to_string(f.dir.join("out/site").join(path)).unwrap();
    // The page list only: on narrow screens the sidebar also repeats the header sections above it, and a section's sidebar opens
    // with a link back to the rest of the site.
    let sidebar = |html: &str| {
      let start = html.find("class=\"site-nav\"").unwrap();
      let end = html[start..].find("</nav>").unwrap();
      let nav = &html[start..start + end];
      let nav = match nav.find("class=\"site-nav-sections\"") {
        Some(sections) => &nav[sections + nav[sections..].find("</ul>").unwrap()..],
        None => nav,
      };
      match nav.find("class=\"site-nav-back\"") {
        Some(back) => nav[back + nav[back..].find("</a>").unwrap()..].to_string(),
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
    // The section's sidebar leads back to the rest of the site, named after it; the rest of the site needs no way back.
    assert!(install.contains("class=\"site-nav-back\" href=\"/about/\""), "{install}");
    assert!(!about.contains("site-nav-back"));
  }

  #[tokio::test]
  async fn renaming_a_page_updates_every_card_that_points_to_it() {
    use cairn_store::{rename_page, set_description};

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
    rename_page(&f.db, target.id, "Undo a release", cairn_core::OldUrls::Redirect).await.unwrap();
    // The pages showing it are pending as edited, besides the renamed page itself.
    let pending: Vec<_> = crate::pending(&f.db).await.unwrap().changes.into_iter().map(|c| (c.title, c.kinds)).collect();
    assert_eq!(
      pending,
      [
        ("Undo a release".to_string(), vec![cairn_core::ChangeKind::Renamed]),
        ("First".to_string(), vec![cairn_core::ChangeKind::Edited]),
        ("Second".to_string(), vec![cairn_core::ChangeKind::Edited]),
      ]
    );
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

    use cairn_assets::{inspect, process, sha256_hex};
    use cairn_core::{AssetStore, AssetVariant, original_key, variant_key};
    use cairn_store::{NewAsset, get_or_create_asset};

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
    use cairn_store::rename_page;

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
    rename_page(&f.db, drafts[0].0.id, "Uno", cairn_core::OldUrls::Redirect).await.unwrap();
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders(), 5 + 4);
    assert!(std::fs::read_to_string(site.join("three/index.html")).unwrap().contains(">Uno<"));
  }

  #[tokio::test]
  async fn new_bundles_render_the_live_release_again_once_and_never_publish_drafts() {
    let f = fixture().await;
    let user = UserId::new();
    let paragraph = |text: &str| doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }] }));

    let mut drafts = Vec::new();
    for title in ["One", "Two", "Three"] {
      let page = create_page(&f.db, user, None, title, Placement::Last).await.unwrap();
      drafts.push((page.id, f.content.save_draft(page.id, paragraph("published text"), None, user).await.unwrap()));
    }
    f.publisher.publish(user).await.unwrap();

    // The same bundles: nothing to do.
    assert!(f.publisher.republish_for_bundles().await.unwrap().is_none());

    // A draft nobody published yet, and a start with other bundles over the same site (a new process: its render counter starts at 0).
    f.content.save_draft(drafts[0].0, paragraph("unpublished draft"), drafts[0].1.base_revision_id, user).await.unwrap();
    let restarted = Publisher::new(PublisherParts {
      db: f.db.clone(),
      content: f.content.clone(),
      assets: Arc::new(LocalAssetStore::new(&f.dir.join("assets")).unwrap()),
      pool: f.publisher.pool.clone(),
      target: PublishTarget::Folder(FolderTarget::new(f.dir.join("out"))),
      bundles: Bundles::new("ssr", b"console.log(2)".to_vec(), b"body{}".to_vec(), b"window.Scalar = {}".to_vec()),
      config: PublishConfig {
        site_url: "https://docs.example.com".into(),
        pagefind: std::env::var("PAGEFIND_BIN").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../node_modules/.bin/pagefind").into()),
        api_hosts: ApiHosts::default(),
      },
      broadcast: LocalBroadcast::new(),
    });

    let release = restarted.republish_for_bundles().await.unwrap().expect("the bundles changed");
    // Three pages and the 404 page, each rendered once, and the draft stayed a draft.
    assert_eq!((release.status, restarted.renders()), (ReleaseStatus::Live, 4));
    let site = f.dir.join("out/site");
    assert!(std::fs::read_to_string(site.join("one/index.html")).unwrap().contains("published text"));

    // Once: the next start with the same bundles renders nothing.
    assert!(restarted.republish_for_bundles().await.unwrap().is_none());
    assert_eq!(restarted.renders(), 4);
  }

  #[tokio::test]
  async fn changing_the_accent_color_renders_every_page_again() {
    use cairn_core::Settings;
    use cairn_store::save_settings;

    let f = fixture().await;
    let user = UserId::new();
    for title in ["One", "Two", "Three"] {
      let page = create_page(&f.db, user, None, title, Placement::Last).await.unwrap();
      f.content
        .save_draft(page.id, doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph" }] })), None, user)
        .await
        .unwrap();
    }

    // Three pages and the 404 page.
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders(), 4);

    save_settings(
      &f.db,
      &Settings {
        accent: "#aa3300".into(),
        ..Settings::default()
      },
    )
    .await
    .unwrap();
    f.publisher.publish(user).await.unwrap();
    assert_eq!(f.publisher.renders(), 8);
    for slug in ["one", "two", "three"] {
      assert!(
        std::fs::read_to_string(f.dir.join(format!("out/site/{slug}/index.html"))).unwrap().contains("--accent-base:#aa3300"),
        "{slug}"
      );
    }
  }

  #[tokio::test]
  async fn each_kind_of_problem_blocks_the_publish_and_names_its_page() {
    use cairn_core::ProblemKind;

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
    cairn_store::delete_page(&f.db, doomed.id, cairn_core::UserId::new()).await.unwrap();

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
  async fn only_the_render_cache_entries_of_the_live_release_are_kept() {
    let f = fixture().await;
    let user = UserId::new();
    let assets = LocalAssetStore::new(&f.dir.join("assets")).unwrap();
    let page = create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();
    create_page(&f.db, user, None, "Other", Placement::Last).await.unwrap();

    let mut base = None;
    for text in ["one", "two", "three"] {
      base = f.content.save_draft(page.id, Document::paragraph(text), base, user).await.unwrap().base_revision_id;
      f.publisher.publish(user).await.unwrap();
    }

    // Two pages and the 404 page: the earlier renders of the edited page are gone.
    let mut cached = assets.list(RENDERS_FOLDER).await.unwrap();
    cached.sort();
    let mut kept: Vec<String> = kept_render_keys(&f.db).await.unwrap().unwrap().into_iter().collect();
    kept.sort();
    assert_eq!((cached.len(), &cached), (3, &kept));
  }

  #[tokio::test]
  async fn editing_a_snippet_renders_exactly_the_pages_that_use_it() {
    use cairn_store::{save_snippet, save_variable};

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
    // Before publishing, the twelve show as edited, and only they.
    let pending = crate::pending(&f.db).await.unwrap();
    let mut edited: Vec<_> = pending.changes.iter().filter(|c| c.kinds == [cairn_core::ChangeKind::Edited]).map(|c| c.title.clone()).collect();
    edited.sort();
    let mut expected: Vec<_> = titles.iter().filter(|(_, uses)| *uses).map(|(t, _)| t.clone()).collect();
    expected.sort();
    assert_eq!((edited, pending.changes.len()), (expected, 12));

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
    use cairn_store::{TermInput, save_term};

    let f = fixture().await;
    let user = UserId::new();
    let rollouts = create_page(&f.db, user, None, "Rollouts", Placement::Last).await.unwrap();
    let term = |term: &str, definition: Document, aliases: &[&str], page: Option<PageId>| TermInput {
      term: term.into(),
      definition,
      aliases: aliases.iter().map(|a| a.to_string()).collect(),
      page,
    };
    // A rich definition: the hover shows its text, the Glossary page the formatting.
    let rich = doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
      { "type": "text", "text": "Switching " },
      { "type": "text", "text": "all", "marks": [{ "type": "bold" }] },
      { "type": "text", "text": " traffic to a new release at once." },
    ] }] }));
    save_term(&f.db, user, &term("Blue-green deploy", rich, &["red-black deploy"], Some(rollouts.id))).await.unwrap();
    save_term(&f.db, user, &term("API", Document::paragraph("A way for programs to talk to each other."), &[], None))
      .await
      .unwrap();
    // Another term cannot take an alias that is already used.
    let taken = save_term(&f.db, user, &term("Red-black", Document::paragraph("Same."), &["Red-black deploy"], None)).await;
    assert!(matches!(&taken, Err(cairn_store::StoreError::Invalid(m)) if m.contains("Blue-green deploy")), "{taken:?}");

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
    // With its formatting, its other words and the page that explains it.
    assert!(glossary.contains("Switching <strong>all</strong> traffic"), "{glossary}");
    assert!(glossary.contains("Also called <em>red-black deploy</em>."), "{glossary}");
    assert!(glossary.contains("href=\"/rollouts/\"") && glossary.contains("Read more about Blue-green deploy"), "{glossary}");

    // Listed for search engines and language models, with a Markdown copy.
    let sitemap = std::fs::read_to_string(site.join("sitemap.xml")).unwrap();
    assert!(sitemap.contains("<loc>https://docs.example.com/glossary/</loc>"), "{sitemap}");
    let llms = std::fs::read_to_string(site.join("llms.txt")).unwrap();
    assert!(llms.contains("- [Glossary](https://docs.example.com/glossary.md)"), "{llms}");
    let markdown = std::fs::read_to_string(site.join("glossary.md")).unwrap();
    assert!(markdown.contains("## API") && markdown.contains("Also called *red-black deploy*."), "{markdown}");
  }

  mod apis {
    use cairn_core::PageKind;
    use cairn_core::{ApiConfig, ApiSource, GitRef};
    use cairn_store::{create_api_page, release_specs, save_api_config};

    use super::*;
    use crate::api::testing::{StandIn, spec};

    fn github(repo: &str, path: &str, asset: Option<&str>) -> ApiSource {
      ApiSource::Github {
        repo: repo.into(),
        path: path.into(),
        git_ref: GitRef::LatestRelease,
        asset: asset.map(Into::into),
        prefix: String::new(),
      }
    }

    /// An API page built from the latest release of two repositories, next to a plain page, and a stand-in that serves them.
    async fn site() -> (Fixture, StandIn, Page, UserId) {
      let github_stand_in = StandIn::start().await;
      github_stand_in.set_latest("acme/orders", "v1.0.0");
      github_stand_in.set_file("/acme/orders/releases/download/v1.0.0/openapi.json", &spec("Orders", "1.0.0", "get", "/orders", "listOrders"));
      github_stand_in.set_latest("acme/billing", "v2.0.0");
      github_stand_in.set_file("/acme/billing/v2.0.0/api/billing.json", &spec("Billing", "2.0.0", "get", "/invoices", "listInvoices"));

      let f = fixture_with(github_stand_in.hosts()).await;
      let user = UserId::new();
      let plain = create_page(&f.db, user, None, "Guide", Placement::Last).await.unwrap();
      f.content
        .save_draft(plain.id, doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph" }] })), None, user)
        .await
        .unwrap();
      let api = create_api_page(&f.db, user, None, "Orders API", Placement::Last).await.unwrap();
      let config = ApiConfig {
        sources: vec![github("acme/orders", "", Some("openapi.json")), github("acme/billing", "api/billing.json", None)],
        ..Default::default()
      };
      save_api_config(&f.db, api.id, &config, user).await.unwrap();

      (f, github_stand_in, api, user)
    }

    fn live_target(f: &Fixture) -> std::path::PathBuf {
      std::fs::read_link(f.dir.join("out/site")).unwrap()
    }

    #[tokio::test]
    async fn the_merged_spec_is_published_and_recorded_with_where_it_came_from() {
      let (f, _github, api, user) = site().await;
      let release = f.publisher.publish(user).await.unwrap();

      let recorded = release_specs(&f.db, release.id).await.unwrap();
      assert_eq!(recorded.len(), 1);
      let spec_file = f.dir.join(format!("out/site/_api/orders-api.{}.json", recorded[0].spec_hash));
      let merged: serde_json::Value = serde_json::from_slice(&std::fs::read(spec_file).unwrap()).unwrap();
      assert!(merged.pointer("/paths/~1orders/get").is_some() && merged.pointer("/paths/~1invoices/get").is_some());

      assert_eq!(recorded[0].page_id, api.id);
      assert_eq!(recorded[0].sources.iter().map(|s| s.tag.as_deref()).collect::<Vec<_>>(), [Some("v1.0.0"), Some("v2.0.0")]);
      assert_eq!(recorded[0].operations.iter().map(|o| o.method.as_str()).collect::<Vec<_>>(), ["GET", "GET"]);
      // Search reads the operations from a hidden block; there is no visible list.
      let html = std::fs::read_to_string(f.dir.join("out/site/orders-api/index.html")).unwrap();
      assert!(html.contains("<div hidden=\"\">") && html.contains("listOrders summary") && html.contains("/invoices"), "{html}");
      // The page has no revision of its own.
      assert_eq!(release.page_count, 2);
    }

    #[tokio::test]
    async fn new_bundles_render_the_published_settings_and_what_was_saved_since_stays_pending() {
      use cairn_core::Settings;
      use cairn_store::{get_api_config, save_settings};

      let (f, _github, api, user) = site().await;
      f.publisher.publish(user).await.unwrap();
      assert!(!crate::pending(&f.db).await.unwrap().settings);

      // Saved, not published: a beta pill on the API page, and another site name.
      let mut config = get_api_config(&f.db, api.id).await.unwrap();
      config.display.beta = true;
      save_api_config(&f.db, api.id, &config, user).await.unwrap();
      let renamed = Settings {
        site_name: "Unpublished name".into(),
        ..Settings::default()
      };
      save_settings(&f.db, &renamed).await.unwrap();
      let is_pending = |pending: &cairn_core::Pending| pending.settings && pending.changes.iter().any(|c| c.page_id == api.id && c.kinds == [cairn_core::ChangeKind::Edited]);
      assert!(is_pending(&crate::pending(&f.db).await.unwrap()));

      // Other bundles at the next start: the live site is rendered again as it was published.
      set_setting(&f.db, BUNDLES_SETTING, "other bundles").await.unwrap();
      f.publisher.republish_for_bundles().await.unwrap().expect("the bundles changed");
      let html = std::fs::read_to_string(f.dir.join("out/site/orders-api/index.html")).unwrap();
      assert!(!html.contains("Unpublished name") && !html.contains("Beta"), "{html}");
      assert!(is_pending(&crate::pending(&f.db).await.unwrap()));

      // The next publish takes them, and nothing is pending any more.
      f.publisher.publish(user).await.unwrap();
      let html = std::fs::read_to_string(f.dir.join("out/site/orders-api/index.html")).unwrap();
      assert!(html.contains("Unpublished name") && html.contains("Beta"));
      let pending = crate::pending(&f.db).await.unwrap();
      assert!(!pending.settings && pending.changes.is_empty(), "{pending:?}");
    }

    #[tokio::test]
    async fn a_link_to_an_operation_the_build_no_longer_has_blocks_the_publish() {
      let (f, github_stand_in, api, user) = site().await;
      let guide = cairn_store::list_pages(&f.db).await.unwrap().into_iter().find(|p| p.kind == PageKind::Page).unwrap();
      let linking = |anchor: &str| {
        doc(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
          { "type": "text", "text": "orders", "marks": [{ "type": "link", "attrs": { "pageId": api.id.to_string(), "anchor": anchor } }] }
        ] }] }))
      };
      let save = |anchor: &'static str| {
        let (f, linking) = (&f, linking(anchor));
        async move {
          let base = f.content.draft(guide.id).await.unwrap().and_then(|d| d.base_revision_id);
          f.content.save_draft(guide.id, linking, base, user).await.unwrap();
        }
      };

      save("tag/default/GET/orders").await;
      f.publisher.publish(user).await.unwrap();

      // Upstream drops the operation: the build has the new spec, and the link no longer leads anywhere.
      github_stand_in.set_file("/acme/orders/releases/download/v1.0.0/openapi.json", &spec("Orders", "1.0.0", "get", "/orders-v2", "listOrders"));
      let blocked = f.publisher.publish(user).await.unwrap_err();
      assert!(
        matches!(&blocked, PublishError::Blocked(problems) if problems[0].page_title == "Guide" && problems[0].message.contains("GET /orders") && problems[0].message.contains("Orders API")),
        "{blocked:?}"
      );
    }

    #[tokio::test]
    async fn an_unreachable_source_fails_the_publish_and_the_previous_release_stays_live() {
      let (f, github_stand_in, _api, user) = site().await;
      f.publisher.publish(user).await.unwrap();
      let live = live_target(&f);
      let releases = std::fs::read_dir(f.dir.join("out/releases")).unwrap().count();

      github_stand_in.remove_file("/acme/billing/v2.0.0/api/billing.json");
      let error = f.publisher.publish(user).await.unwrap_err();

      assert!(
        matches!(&error, PublishError::Api(problems) if problems[0].page_title == "Orders API" && problems[0].message.contains("acme/billing api/billing.json: HTTP 404")),
        "{error:?}"
      );
      assert_eq!(live_target(&f), live);
      assert_eq!(std::fs::read_dir(f.dir.join("out/releases")).unwrap().count(), releases);
    }

    #[tokio::test]
    async fn a_new_upstream_release_is_picked_up_and_only_the_api_page_renders_again() {
      let (f, github_stand_in, _api, user) = site().await;
      let first = f.publisher.publish(user).await.unwrap();
      let rendered = f.publisher.renders();

      // Nothing changed upstream: the same spec, the same hash, and every page comes from the render cache.
      let again = f.publisher.publish(user).await.unwrap();
      assert_eq!(f.publisher.renders(), rendered);
      assert_eq!(release_specs(&f.db, again.id).await.unwrap()[0].spec_hash, release_specs(&f.db, first.id).await.unwrap()[0].spec_hash);

      github_stand_in.set_latest("acme/orders", "v1.1.0");
      github_stand_in.set_file("/acme/orders/releases/download/v1.1.0/openapi.json", &spec("Orders", "1.1.0", "get", "/orders", "listOrdersV2"));
      let next = f.publisher.publish(user).await.unwrap();

      let recorded = release_specs(&f.db, next.id).await.unwrap();
      assert_ne!(recorded[0].spec_hash, release_specs(&f.db, first.id).await.unwrap()[0].spec_hash);
      assert_eq!(recorded[0].sources[0].tag.as_deref(), Some("v1.1.0"));
      assert_eq!(f.publisher.renders(), rendered + 1);
    }

    #[tokio::test]
    async fn publishing_again_for_new_bundles_uses_the_recorded_specs_and_never_the_network() {
      let (f, github_stand_in, _api, user) = site().await;
      let first = f.publisher.publish(user).await.unwrap();
      let recorded = release_specs(&f.db, first.id).await.unwrap();
      let hits = github_stand_in.hits();

      // Upstream moves on, and the bundles change.
      github_stand_in.set_latest("acme/orders", "v9.9.9");
      let restarted = Publisher::new(PublisherParts {
        db: f.db.clone(),
        content: f.content.clone(),
        assets: Arc::new(LocalAssetStore::new(&f.dir.join("assets")).unwrap()),
        pool: f.publisher.pool.clone(),
        target: PublishTarget::Folder(FolderTarget::new(f.dir.join("out"))),
        bundles: Bundles::new("ssr", b"console.log(2)".to_vec(), b"body{}".to_vec(), b"window.Scalar = {}".to_vec()),
        config: PublishConfig {
          site_url: "https://docs.example.com".into(),
          pagefind: std::env::var("PAGEFIND_BIN").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../node_modules/.bin/pagefind").into()),
          api_hosts: github_stand_in.hosts(),
        },
        broadcast: LocalBroadcast::new(),
      });
      let release = restarted.republish_for_bundles().await.unwrap().expect("the bundles changed");

      assert_eq!(github_stand_in.hits(), hits);
      let again = release_specs(&f.db, release.id).await.unwrap();
      assert_eq!(again, recorded);
      assert!(f.dir.join(format!("out/site/_api/orders-api.{}.json", again[0].spec_hash)).exists());
    }
  }
}
