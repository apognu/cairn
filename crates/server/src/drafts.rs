use axum::{
  Json,
  extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{ContentStore, Document, PageId, RevisionId};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(get_draft, save_draft))
}

#[derive(Serialize, ToSchema)]
struct DraftResponse {
  doc: Document,
  /// The revision this draft is at; the next save must name it.
  base_revision_id: Option<RevisionId>,
}

#[derive(Deserialize, ToSchema)]
struct SaveDraft {
  doc: Document,
  /// The `base_revision_id` of the draft this edit started from, or null if the page had no draft.
  base_revision_id: Option<RevisionId>,
}

/// The latest saved edits of a page (an empty document if it has none yet).
#[utoipa::path(get, path = "/api/pages/{id}/draft", operation_id = "getDraft", params(("id" = PageId, Path)), responses(
  (status = 200, body = DraftResponse),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_draft(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<DraftResponse>, AppError> {
  // The page must exist even when it has no draft yet.
  if !yourdocs_store::list_pages(&state.db).await?.iter().any(|p| p.id == id) {
    return Err(AppError::Core(yourdocs_core::CoreError::NotFound));
  }

  let response = match state.content.draft(id).await? {
    Some(draft) => DraftResponse {
      doc: draft.doc,
      base_revision_id: draft.base_revision_id,
    },
    None => DraftResponse {
      doc: Document::empty(),
      base_revision_id: None,
    },
  };

  Ok(Json(response))
}

/// Saves a draft. Refused with 409 when `base_revision_id` is not the page's latest revision.
#[utoipa::path(put, path = "/api/pages/{id}/draft", operation_id = "saveDraft", params(("id" = PageId, Path)), request_body = SaveDraft, responses(
  (status = 200, body = DraftResponse),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "the page changed since the revision this save was based on"),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn save_draft(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>, Json(body): Json<SaveDraft>) -> Result<Json<DraftResponse>, AppError> {
  if body.doc.0.kind != "doc" {
    return Err(AppError::Invalid(vec!["the root node must have type \"doc\"".into()]));
  }

  let draft = state.content.save_draft(id, body.doc, body.base_revision_id, user).await?;

  Ok(Json(DraftResponse {
    doc: draft.doc,
    base_revision_id: draft.base_revision_id,
  }))
}

#[cfg(test)]
mod tests {
  use yourdocs_auth::local_user_id;
  use yourdocs_core::Placement;

  use super::*;
  use crate::app::test_state;

  #[tokio::test]
  async fn a_stale_save_is_a_conflict() {
    let state = test_state().await;
    let page = yourdocs_store::create_page(&state.db, local_user_id(), None, "A", Placement::Last).await.unwrap().id;
    let save = |base| SaveDraft {
      doc: Document::empty(),
      base_revision_id: base,
    };

    let Json(first) = save_draft(State(state.clone()), CurrentUser(local_user_id()), Path(page), Json(save(None))).await.unwrap();
    let _ = save_draft(State(state.clone()), CurrentUser(local_user_id()), Path(page), Json(save(first.base_revision_id)))
      .await
      .unwrap();

    let stale = save_draft(State(state), CurrentUser(local_user_id()), Path(page), Json(save(first.base_revision_id))).await;
    assert!(matches!(stale, Err(AppError::Core(yourdocs_core::CoreError::Conflict { .. }))));
  }
}
