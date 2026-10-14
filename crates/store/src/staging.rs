//! Everything a publish reads, taken at one moment, so its checks and its build agree with each other.

use std::collections::HashMap;

use cairn_core::{ApiConfig, Document, Library, Page, PageId, Redirect, RevisionId, Settings};

use crate::{
  LiveRelease, ReleaseSpec, StoreError, StoredLink, all_links, connect::snapshot_tx, draft_docs, draft_revisions, list_api_configs, list_pages, list_redirects, live_release, load_library,
  load_settings, release_specs,
};

/// The shared staging state and the live release it is compared with.
pub struct Staging {
  pub pages: Vec<Page>,
  pub redirects: Vec<Redirect>,
  /// The latest saved revision of every page that has a draft.
  pub revisions: HashMap<PageId, RevisionId>,
  /// The saved document of every page that has a draft, the one in `revisions`.
  pub docs: HashMap<PageId, Document>,
  pub links: Vec<StoredLink>,
  pub api_configs: HashMap<PageId, ApiConfig>,
  pub settings: Settings,
  pub library: Library,
  pub live: Option<LiveRelease>,
  /// The API specs the live release was built with.
  pub live_specs: Vec<ReleaseSpec>,
}

pub async fn staging(db: &sea_orm::DatabaseConnection) -> Result<Staging, StoreError> {
  let tx = snapshot_tx(db).await?;

  let live = live_release(&tx).await?;
  let live_specs = match &live {
    Some(live) => release_specs(&tx, live.id).await?,
    None => Vec::new(),
  };
  let staging = Staging {
    pages: list_pages(&tx).await?,
    redirects: list_redirects(&tx).await?,
    revisions: draft_revisions(&tx).await?,
    docs: draft_docs(&tx).await?,
    links: all_links(&tx).await?,
    api_configs: list_api_configs(&tx).await?,
    settings: load_settings(&tx).await?,
    library: load_library(&tx).await?,
    live,
    live_specs,
  };
  tx.commit().await?;

  Ok(staging)
}
