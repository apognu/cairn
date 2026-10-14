use axum::{
  Json,
  extract::{Path, State},
};
use cairn_auth::CurrentUser;
use cairn_core::{ContentStore, Document, LockToken, PageId, RevisionId};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

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
  /// The token of the page's lock, which the saver must hold.
  lock_token: LockToken,
}

/// The latest saved edits of a page (an empty document if it has none yet).
#[utoipa::path(get, path = "/api/pages/{id}/draft", operation_id = "getDraft", params(("id" = PageId, Path)), responses(
  (status = 200, body = DraftResponse),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_draft(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<DraftResponse>, AppError> {
  // The page must exist even when it has no draft yet.
  if !cairn_store::list_pages(&state.db).await?.iter().any(|p| p.id == id) {
    return Err(AppError::Core(cairn_core::CoreError::NotFound));
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

/// Saves a draft, which also renews the lock. Refused with 409 when `base_revision_id` is not the page's latest revision, and with 423 or
/// 410 when the saver does not hold the page's lock.
#[utoipa::path(put, path = "/api/pages/{id}/draft", operation_id = "saveDraft", params(("id" = PageId, Path)), request_body = SaveDraft, responses(
  (status = 200, body = DraftResponse),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "the page changed since the revision this save was based on"),
  (status = 410, body = ErrorBody, description = "the lock was released"),
  (status = 422, body = ErrorBody),
  (status = 423, body = ErrorBody, description = "someone else is editing the page"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn save_draft(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>, Json(body): Json<SaveDraft>) -> Result<Json<DraftResponse>, AppError> {
  if body.doc.0.kind != "doc" {
    return Err(AppError::Invalid(vec!["the root node must have type \"doc\"".into()]));
  }

  crate::locks::keep(&state, id, body.lock_token, user, jiff::Timestamp::now()).await?;
  let draft = state.content.save_draft(id, body.doc, body.base_revision_id, user).await?;

  Ok(Json(DraftResponse {
    doc: draft.doc,
    base_revision_id: draft.base_revision_id,
  }))
}

#[cfg(test)]
mod tests {
  use cairn_auth::local_user_id;
  use cairn_core::Placement;

  use super::*;
  use crate::app::test_state;

  #[tokio::test]
  async fn a_stale_save_is_a_conflict() {
    let state = test_state().await;
    let page = cairn_store::create_page(&state.db, local_user_id(), None, "A", Placement::Last).await.unwrap().id;
    let (_, token) = cairn_store::acquire_lock(&state.db, page, local_user_id(), jiff::Timestamp::now()).await.unwrap();
    let save = |base| SaveDraft {
      doc: Document::empty(),
      base_revision_id: base,
      lock_token: token,
    };

    let Json(first) = save_draft(State(state.clone()), CurrentUser(local_user_id()), Path(page), Json(save(None))).await.unwrap();
    let _ = save_draft(State(state.clone()), CurrentUser(local_user_id()), Path(page), Json(save(first.base_revision_id)))
      .await
      .unwrap();

    let stale = save_draft(State(state), CurrentUser(local_user_id()), Path(page), Json(save(first.base_revision_id))).await;
    assert!(matches!(stale, Err(AppError::Core(cairn_core::CoreError::Conflict { .. }))));
  }

  #[tokio::test]
  async fn only_the_lock_holder_saves() {
    let state = test_state().await;
    let (alice, bob) = (local_user_id(), cairn_core::UserId::new());
    let page = cairn_store::create_page(&state.db, alice, None, "A", Placement::Last).await.unwrap().id;
    let (_, token) = cairn_store::acquire_lock(&state.db, page, alice, jiff::Timestamp::now()).await.unwrap();
    let save = |lock_token| SaveDraft {
      doc: Document::empty(),
      base_revision_id: None,
      lock_token,
    };

    let other = save_draft(State(state.clone()), CurrentUser(bob), Path(page), Json(save(cairn_core::LockToken::new()))).await;
    assert!(matches!(other, Err(AppError::Locked(m)) if m == "Local user is editing this page"));

    cairn_store::release_lock(&state.db, page, token).await.unwrap();
    let released = save_draft(State(state), CurrentUser(alice), Path(page), Json(save(token))).await;
    assert!(matches!(released, Err(AppError::LockLost)));
  }
}
