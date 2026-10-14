//! Page locks over HTTP: the editor takes one on opening a page, renews it while its author works, and gives it back on leaving.

use axum::{
  Json,
  extract::{Path, Query, State},
  http::StatusCode,
};
use cairn_auth::CurrentUser;
use cairn_core::{ContentStore, LockToken, PageId, PageKind, PageLock, UserId};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
  me::display_name,
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(list)).routes(routes!(get_lock, acquire, renew, release))
}

/// A lock as the editor shows it.
#[derive(Serialize, ToSchema)]
pub(crate) struct LockView {
  page_id: PageId,
  user_id: UserId,
  user_name: String,
  /// Held by the user asking.
  mine: bool,
  #[schema(value_type = String, format = DateTime)]
  acquired_at: Timestamp,
  /// When the holder's editor last renewed the lock: it was open and in use then.
  #[schema(value_type = String, format = DateTime)]
  renewed_at: Timestamp,
  #[schema(value_type = String, format = DateTime)]
  expires_at: Timestamp,
  /// When the page's draft was last saved, by anyone.
  #[schema(value_type = Option<String>, format = DateTime)]
  last_saved_at: Option<Timestamp>,
}

#[derive(Serialize, ToSchema)]
struct AcquiredLock {
  /// Sent with every save and renewal.
  token: LockToken,
  lock: LockView,
}

#[derive(Deserialize, ToSchema)]
struct RenewLock {
  token: LockToken,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct ReleaseQuery {
  token: LockToken,
}

pub(crate) async fn view(state: &AppState, lock: PageLock, me: UserId) -> Result<LockView, AppError> {
  let last_saved_at = state.content.draft(lock.page_id).await?.map(|d| d.updated_at);

  Ok(LockView {
    page_id: lock.page_id,
    user_name: display_name(state, lock.user_id).await?,
    mine: lock.user_id == me,
    user_id: lock.user_id,
    acquired_at: lock.acquired_at,
    renewed_at: lock.renewed_at,
    expires_at: lock.expires_at,
    last_saved_at,
  })
}

/// A held lock as an error that names the holder.
pub(crate) async fn held(state: &AppState, lock: PageLock, me: UserId) -> AppError {
  let name = match lock.user_id == me {
    true => "You are".to_string(),
    false => match display_name(state, lock.user_id).await {
      Ok(name) => format!("{name} is"),
      Err(error) => return error,
    },
  };
  let place = if lock.user_id == me { " in another tab" } else { "" };

  AppError::Locked(format!("{name} editing this page{place}"))
}

/// Every page someone is editing now.
#[utoipa::path(get, path = "/api/locks", operation_id = "listLocks", responses((status = 200, body = Vec<LockView>)))]
#[instrument(skip_all)]
async fn list(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Result<Json<Vec<LockView>>, AppError> {
  let mut views = Vec::new();
  for lock in cairn_store::live_locks(&state.db, Timestamp::now()).await? {
    views.push(view(&state, lock, user).await?);
  }

  Ok(Json(views))
}

/// Who is editing the page, if anyone.
#[utoipa::path(get, path = "/api/pages/{id}/lock", operation_id = "getLock", params(("id" = PageId, Path)), responses((status = 200, body = Option<LockView>)))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_lock(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>) -> Result<Json<Option<LockView>>, AppError> {
  let lock = match cairn_store::page_lock(&state.db, id, Timestamp::now()).await? {
    Some(lock) => Some(view(&state, lock, user).await?),
    None => None,
  };

  Ok(Json(lock))
}

/// Starts editing: takes the page's lock with a new token. Refused with 423 while someone else holds it.
#[utoipa::path(post, path = "/api/pages/{id}/lock", operation_id = "acquireLock", params(("id" = PageId, Path)), responses(
  (status = 200, body = AcquiredLock),
  (status = 404, body = ErrorBody),
  (status = 423, body = ErrorBody, description = "someone else is editing the page"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn acquire(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>) -> Result<Json<AcquiredLock>, AppError> {
  if !cairn_store::list_pages(&state.db).await?.iter().any(|p| p.id == id && p.kind == PageKind::Page) {
    return Err(AppError::NotFound);
  }

  match cairn_store::acquire_lock(&state.db, id, user, Timestamp::now()).await {
    Ok((lock, token)) => {
      tracing::info!(page_id = %id, user_id = %user, "page locked");
      Ok(Json(AcquiredLock {
        token,
        lock: view(&state, lock, user).await?,
      }))
    }
    Err(cairn_store::StoreError::LockHeld(lock)) => Err(held(&state, lock, user).await),
    Err(error) => Err(error.into()),
  }
}

/// Keeps the lock while its holder works. Refused with 423 when someone else took the page, and 410 when the lock was given up.
#[utoipa::path(put, path = "/api/pages/{id}/lock", operation_id = "renewLock", params(("id" = PageId, Path)), request_body = RenewLock, responses(
  (status = 200, body = LockView),
  (status = 410, body = ErrorBody, description = "the lock was released"),
  (status = 423, body = ErrorBody, description = "someone else is editing the page"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn renew(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>, Json(body): Json<RenewLock>) -> Result<Json<LockView>, AppError> {
  let lock = keep(&state, id, body.token, user).await?;

  Ok(Json(view(&state, lock, user).await?))
}

/// Renews the lock `token` names, as a renewal or a save does.
pub(crate) async fn keep(state: &AppState, page: PageId, token: LockToken, user: UserId) -> Result<PageLock, AppError> {
  match cairn_store::renew_lock(&state.db, page, token, Timestamp::now()).await {
    Ok(lock) => Ok(lock),
    Err(cairn_store::StoreError::LockHeld(lock)) => Err(held(state, lock, user).await),
    Err(cairn_store::StoreError::NotFound) => Err(AppError::LockLost),
    Err(error) => Err(error.into()),
  }
}

/// Stops editing. The editor calls it when the page is left or after a long idle time; a token that lost the lock changes nothing.
#[utoipa::path(delete, path = "/api/pages/{id}/lock", operation_id = "releaseLock", params(("id" = PageId, Path), ReleaseQuery), responses((status = 204)))]
#[instrument(skip_all, fields(page_id = %id))]
async fn release(State(state): State<AppState>, Path(id): Path<PageId>, Query(query): Query<ReleaseQuery>) -> Result<StatusCode, AppError> {
  cairn_store::release_lock(&state.db, id, query.token).await?;

  Ok(StatusCode::NO_CONTENT)
}
