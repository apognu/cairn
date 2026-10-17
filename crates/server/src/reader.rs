//! Where readers find a page: on the live site, and in the latest preview.

use axum::{
  Json,
  extract::{Path, State},
};
use cairn_core::{PageId, ReleaseId, page_urls};
use cairn_store::{draft_revisions, latest_preview_release, list_pages, live_release, release_state_of};
use jiff::Timestamp;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{app::AppState, error::AppError};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(reader))
}

/// A page's addresses in the published releases, for the editor's "View as reader" and "See live version".
#[derive(Serialize, ToSchema)]
struct ReaderLinks {
  /// The page on the live site; none before it was first published.
  live: Option<String>,
  /// The page in the newest preview; none when there is no preview, or the page is not in it.
  preview: Option<PreviewLink>,
}

#[derive(Serialize, ToSchema)]
struct PreviewLink {
  url: String,
  release_id: ReleaseId,
  #[schema(value_type = String, format = DateTime)]
  created_at: Timestamp,
  /// The page changed since the preview was built: its saved draft, or its address.
  outdated: bool,
}

/// Where a page can be read now: live, and in the latest preview. Each address comes from that release's own tree, so a page moved or
/// renamed since still opens where the release has it.
#[utoipa::path(get, path = "/api/pages/{id}/reader", operation_id = "readerLinks", params(("id" = PageId, Path)), responses(
  (status = 200, body = ReaderLinks),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn reader(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<ReaderLinks>, AppError> {
  let live = live_release(&state.db)
    .await?
    .and_then(|release| page_urls(&release.pages).remove(&id))
    .map(|path| format!("{}{path}", state.publisher.site_url()));

  let mut preview = None;
  if let Some(release) = latest_preview_release(&state.db).await? {
    let frozen = release_state_of(&state.db, release.id).await?;
    if let Some(path) = page_urls(&frozen.pages).remove(&id) {
      // A self-hosted preview is served by this server, beside the editor; other targets give it an address of its own.
      let base = match &release.preview_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => format!("/_preview/{}", release.id),
      };
      let moved = page_urls(&list_pages(&state.db).await?).get(&id) != Some(&path);
      let edited = draft_revisions(&state.db).await?.get(&id) != frozen.revisions.get(&id);

      preview = Some(PreviewLink {
        url: format!("{base}{path}"),
        release_id: release.id,
        created_at: release.created_at,
        outdated: moved || edited,
      });
    }
  }

  Ok(Json(ReaderLinks { live, preview }))
}
