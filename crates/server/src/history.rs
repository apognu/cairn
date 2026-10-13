use axum::{
  Json,
  extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{ContentStore, Document, Page, PageId, RevisionId, UserId};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new()
    .routes(routes!(history))
    .routes(routes!(revision))
    .routes(routes!(restore_revision))
    .routes(routes!(trash))
    .routes(routes!(restore_from_trash))
}

/// One saved version of a page.
#[derive(Serialize, ToSchema)]
struct Version {
  id: RevisionId,
  author_id: UserId,
  #[schema(value_type = String, format = DateTime)]
  created_at: jiff::Timestamp,
  title: String,
  /// When the live site last got this version, if it ever did.
  #[schema(value_type = Option<String>, format = DateTime)]
  published_at: Option<jiff::Timestamp>,
}

/// The saved versions of a page, newest first.
#[utoipa::path(get, path = "/api/pages/{id}/history", operation_id = "getHistory", params(("id" = PageId, Path)), responses((status = 200, body = Vec<Version>), (status = 404, body = ErrorBody)))]
#[instrument(skip_all, fields(page_id = %id))]
async fn history(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<Vec<Version>>, AppError> {
  if !yourdocs_store::list_pages(&state.db).await?.iter().any(|p| p.id == id) {
    return Err(AppError::NotFound);
  }

  Ok(Json(
    yourdocs_store::page_history(&state.db, id)
      .await?
      .into_iter()
      .map(|h| Version {
        id: h.id,
        author_id: h.author,
        created_at: h.created_at,
        title: h.title,
        published_at: h.released_in.map(|(_, at)| at),
      })
      .collect(),
  ))
}

#[derive(Serialize, ToSchema)]
struct VersionContent {
  doc: Document,
  title: String,
}

/// The content of one saved version.
#[utoipa::path(get, path = "/api/revisions/{id}", operation_id = "getRevision", params(("id" = RevisionId, Path)), responses((status = 200, body = VersionContent), (status = 404, body = ErrorBody)))]
#[instrument(skip_all, fields(revision = %id))]
async fn revision(State(state): State<AppState>, Path(id): Path<RevisionId>) -> Result<Json<VersionContent>, AppError> {
  let revision = state.content.revision(id).await?;

  Ok(Json(VersionContent {
    doc: revision.doc,
    title: revision.title,
  }))
}

#[derive(Serialize, ToSchema)]
struct RestoredDraft {
  /// The new version the draft now is.
  base_revision_id: Option<RevisionId>,
}

#[derive(Deserialize, ToSchema)]
struct Restore {
  revision_id: RevisionId,
}

/// Makes an older version the page's draft. The draft it replaces stays in the history, so the restore can be undone.
#[utoipa::path(post, path = "/api/pages/{id}/restore", operation_id = "restoreRevision", params(("id" = PageId, Path)), request_body = Restore, responses(
  (status = 200, body = RestoredDraft),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn restore_revision(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>, Json(body): Json<Restore>) -> Result<Json<RestoredDraft>, AppError> {
  let revision = state.content.revision(body.revision_id).await?;
  if revision.page_id != id {
    return Err(AppError::NotFound);
  }

  let base = state.content.draft(id).await?.and_then(|d| d.base_revision_id);

  let draft = state.content.save_draft(id, revision.doc, base, user).await?;

  Ok(Json(RestoredDraft {
    base_revision_id: draft.base_revision_id,
  }))
}

/// A deleted page waiting in the trash.
#[derive(Serialize, ToSchema)]
struct TrashedPage {
  id: PageId,
  title: String,
  /// The URL it had.
  old_url: String,
  #[schema(value_type = String, format = DateTime)]
  deleted_at: jiff::Timestamp,
  /// How many pages were deleted with it.
  subpages: usize,
}

/// Deleted pages, most recent first.
#[utoipa::path(get, path = "/api/trash", operation_id = "listTrash", responses((status = 200, body = Vec<TrashedPage>)))]
#[instrument(skip_all)]
async fn trash(State(state): State<AppState>) -> Result<Json<Vec<TrashedPage>>, AppError> {
  Ok(Json(
    yourdocs_store::list_trash(&state.db)
      .await?
      .into_iter()
      .map(|t| TrashedPage {
        id: t.page.id,
        title: t.page.title,
        old_url: t.old_url,
        deleted_at: t.deleted_at,
        subpages: t.subpages,
      })
      .collect(),
  ))
}

#[derive(Serialize, ToSchema)]
struct Restored {
  page: Page,
  /// False when the old parent was gone or the old URL segment taken, so the page came back somewhere else or under another URL.
  old_place_kept: bool,
}

/// Brings a page back from the trash, with the pages deleted with it.
#[utoipa::path(post, path = "/api/trash/{id}/restore", operation_id = "restoreFromTrash", params(("id" = PageId, Path)), responses((status = 200, body = Restored), (status = 404, body = ErrorBody)))]
#[instrument(skip_all, fields(page_id = %id))]
async fn restore_from_trash(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<Restored>, AppError> {
  let (page, old_place_kept) = yourdocs_store::restore_page(&state.db, id).await?;

  Ok(Json(Restored { page, old_place_kept }))
}

#[cfg(test)]
mod tests {
  use yourdocs_auth::local_user_id;
  use yourdocs_core::Placement;

  use super::*;
  use crate::app::test_state;

  #[tokio::test]
  async fn restoring_a_version_makes_a_new_draft_and_keeps_the_replaced_one() {
    let state = test_state().await;
    let user = local_user_id();
    let page = yourdocs_store::create_page(&state.db, user, None, "Page", Placement::Last).await.unwrap();
    let doc = |text: &str| -> Document { serde_json::from_value(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }] })).unwrap() };

    let first = state.content.save_draft(page.id, doc("one"), None, user).await.unwrap();
    state.content.save_draft(page.id, doc("two"), first.base_revision_id, user).await.unwrap();
    let before = history(State(state.clone()), Path(page.id)).await.unwrap().0;
    assert_eq!(before.len(), 2);

    // Restore the oldest version: a third version appears, with the old content; the replaced draft is still listed.
    let oldest = before.last().unwrap().id;
    let _ = restore_revision(State(state.clone()), CurrentUser(user), Path(page.id), Json(Restore { revision_id: oldest }))
      .await
      .unwrap();

    let after = history(State(state.clone()), Path(page.id)).await.unwrap().0;
    assert_eq!(after.len(), 3);
    let draft = state.content.draft(page.id).await.unwrap().unwrap();
    assert!(serde_json::to_string(&draft.doc).unwrap().contains("one"));
    for version in &before {
      assert!(after.iter().any(|v| v.id == version.id), "a version went missing");
    }
  }
}
