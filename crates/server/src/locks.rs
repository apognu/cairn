//! Page locks over HTTP: the editor takes one on opening a page, renews it while its author works, and gives it back on leaving.

use std::convert::Infallible;

use axum::{
  Json,
  extract::{Path, Query, State},
  http::StatusCode,
  response::{
    Sse,
    sse::{Event as SseEvent, KeepAlive},
  },
};
use cairn_auth::CurrentUser;
use cairn_core::{ContentStore, Event, LOCK_IDLE, LockToken, PageId, PageKind, PageLock, ReleaseId, UserId};
use futures_util::{Stream, StreamExt};
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
  me::display_name,
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new()
    .routes(routes!(list))
    .routes(routes!(get_lock, acquire, renew, release))
    .routes(routes!(take_over))
    .routes(routes!(events))
}

/// A lock as the editor shows it.
#[derive(Serialize, ToSchema)]
pub(crate) struct LockView {
  page_id: PageId,
  /// The page's title now; empty when the page is gone.
  page_title: String,
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
  /// When the holder last typed or clicked in the page.
  #[schema(value_type = String, format = DateTime)]
  active_at: Timestamp,
  /// The holder has done nothing for a while: taking over does not need forcing.
  idle: bool,
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
  /// How long ago, in milliseconds, the holder last typed or clicked in the editor.
  #[serde(default)]
  idle_ms: u64,
}

#[derive(Deserialize, ToSchema)]
struct AcquireLock {
  /// How long ago, in milliseconds, the user last typed or clicked in the tab asking; absent when they have not yet.
  idle_ms: Option<u64>,
}

#[derive(Deserialize, ToSchema)]
struct TakeOver {
  /// Take the page even though its holder is still active.
  #[serde(default)]
  force: bool,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct ReleaseQuery {
  token: LockToken,
}

pub(crate) async fn view(state: &AppState, lock: PageLock, me: UserId) -> Result<LockView, AppError> {
  let last_saved_at = state.content.draft(lock.page_id).await?.map(|d| d.updated_at);
  let page_title = cairn_store::list_pages(&state.db)
    .await?
    .into_iter()
    .find(|p| p.id == lock.page_id)
    .map(|p| p.title)
    .unwrap_or_default();

  Ok(LockView {
    page_id: lock.page_id,
    page_title,
    user_name: display_name(state, lock.user_id).await?,
    mine: lock.user_id == me,
    user_id: lock.user_id,
    acquired_at: lock.acquired_at,
    renewed_at: lock.renewed_at,
    expires_at: lock.expires_at,
    active_at: lock.active_at,
    idle: lock.active_at < Timestamp::now() - LOCK_IDLE,
    last_saved_at,
  })
}

fn millis(ms: u64) -> SignedDuration {
  SignedDuration::from_millis(i64::try_from(ms).unwrap_or(i64::MAX))
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
async fn list(State(state): State<AppState>, CurrentUser(user, _): CurrentUser) -> Result<Json<Vec<LockView>>, AppError> {
  let mut views = Vec::new();
  for lock in cairn_store::live_locks(&state.db, Timestamp::now()).await? {
    views.push(view(&state, lock, user).await?);
  }

  Ok(Json(views))
}

/// Who is editing the page, if anyone.
#[utoipa::path(get, path = "/api/pages/{id}/lock", operation_id = "getLock", params(("id" = PageId, Path)), responses((status = 200, body = Option<LockView>)))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_lock(State(state): State<AppState>, CurrentUser(user, _): CurrentUser, Path(id): Path<PageId>) -> Result<Json<Option<LockView>>, AppError> {
  let lock = match cairn_store::page_lock(&state.db, id, Timestamp::now()).await? {
    Some(lock) => Some(view(&state, lock, user).await?),
    None => None,
  };

  Ok(Json(lock))
}

/// Starts editing: takes the page's lock with a new token. Refused with 423 while someone else holds it.
#[utoipa::path(post, path = "/api/pages/{id}/lock", operation_id = "acquireLock", params(("id" = PageId, Path)), request_body = Option<AcquireLock>, responses(
  (status = 200, body = AcquiredLock),
  (status = 404, body = ErrorBody),
  (status = 423, body = ErrorBody, description = "someone else is editing the page"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn acquire(State(state): State<AppState>, CurrentUser(user, _): CurrentUser, Path(id): Path<PageId>, body: Option<Json<AcquireLock>>) -> Result<Json<AcquiredLock>, AppError> {
  if !cairn_store::list_pages(&state.db).await?.iter().any(|p| p.id == id && p.kind == PageKind::Page) {
    return Err(AppError::NotFound);
  }

  let now = Timestamp::now();
  let active_at = body.and_then(|Json(b)| b.idle_ms).map(|idle| now.checked_sub(millis(idle)).unwrap_or(Timestamp::MIN));
  match cairn_store::acquire_lock(&state.db, id, user, now, active_at).await {
    Ok((lock, token)) => {
      tracing::info!(page_id = %id, user_id = %user, "page locked");
      state.publisher.notify(Event::LockChanged { page: id });
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
async fn renew(State(state): State<AppState>, CurrentUser(user, _): CurrentUser, Path(id): Path<PageId>, Json(body): Json<RenewLock>) -> Result<Json<LockView>, AppError> {
  let lock = keep(&state, id, body.token, user, Timestamp::now().checked_sub(millis(body.idle_ms)).unwrap_or(Timestamp::MIN)).await?;

  Ok(Json(view(&state, lock, user).await?))
}

/// Renews the lock `token` names, as a renewal or a save (active now) does.
pub(crate) async fn keep(state: &AppState, page: PageId, token: LockToken, user: UserId, active_at: Timestamp) -> Result<PageLock, AppError> {
  match cairn_store::renew_lock(&state.db, page, token, Timestamp::now(), active_at).await {
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
  state.publisher.notify(Event::LockChanged { page: id });

  Ok(StatusCode::NO_CONTENT)
}

/// Takes the page from someone who left it idle (or, with `force`, from someone still at work). Their editor is told at once, stops,
/// and keeps what it has not saved in their browser. Refused with 409 while the holder is active and `force` is not set.
#[utoipa::path(post, path = "/api/pages/{id}/lock/take-over", operation_id = "takeOverLock", params(("id" = PageId, Path)), request_body = TakeOver, responses(
  (status = 200, body = AcquiredLock),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "the holder is still active"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn take_over(State(state): State<AppState>, CurrentUser(user, _): CurrentUser, Path(id): Path<PageId>, Json(body): Json<TakeOver>) -> Result<Json<AcquiredLock>, AppError> {
  if !cairn_store::list_pages(&state.db).await?.iter().any(|p| p.id == id && p.kind == PageKind::Page) {
    return Err(AppError::NotFound);
  }

  let (lock, token, from) = cairn_store::take_over_lock(&state.db, id, user, Timestamp::now(), body.force).await?;
  if let Some(from) = from {
    tracing::info!(page_id = %id, user_id = %user, from = %from, forced = body.force, "page lock taken over");
    state.publisher.notify(Event::LockTakenOver { page: id, from });
  }
  state.publisher.notify(Event::LockChanged { page: id });

  Ok(Json(AcquiredLock {
    token,
    lock: view(&state, lock, user).await?,
  }))
}

/// What open editors are told about, as server-sent events: one JSON object each, tagged by `type`.
#[derive(Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EditorEvent {
  /// Someone took, renewed into, or gave back a page's lock: read it again.
  LockChanged { page_id: PageId },
  /// `from` lost the page to someone else.
  LockTakenOver { page_id: PageId, from: UserId },
  /// A release went live.
  Published { release_id: ReleaseId },
}

/// Server-sent events for open editors: lock changes and new releases. They carry IDs only; the editor reads what it needs again. A
/// stream that falls behind skips the oldest events, so a reconnecting editor reloads its state.
#[utoipa::path(get, path = "/api/events", operation_id = "editorEvents", responses(
  (status = 200, content_type = "text/event-stream", body = EditorEvent),
))]
#[instrument(skip_all)]
async fn events(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
  let stopping = state.until_stopping();
  let stream = BroadcastStream::new(state.publisher.subscribe()).take_until(stopping).filter_map(|event| async move {
    let event = match event.ok()? {
      Event::LockChanged { page } => EditorEvent::LockChanged { page_id: page },
      Event::LockTakenOver { page, from } => EditorEvent::LockTakenOver { page_id: page, from },
      Event::Published { release } => EditorEvent::Published { release_id: release },
      Event::PublishProgress { .. } => return None,
    };
    SseEvent::default().json_data(event).ok().map(Ok)
  });

  Sse::new(stream).keep_alive(KeepAlive::default())
}
