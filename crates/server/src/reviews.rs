//! Page owners, review dates and the report of pages due for a review.

use axum::{
  Json,
  extract::{Path, State},
};
use cairn_auth::{CurrentUser, local_user_id};
use cairn_core::{PageId, PageReview, StalePage, User};
use cairn_store::{get_user, list_users, page_review, set_page_review, stale_pages};
use jiff::Zoned;
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(get_review, set_review)).routes(routes!(users)).routes(routes!(stale))
}

/// A page's owner and review date.
#[utoipa::path(get, path = "/api/pages/{id}/review", operation_id = "getPageReview", params(("id" = PageId, Path)), responses(
  (status = 200, body = PageReview),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_review(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<PageReview>, AppError> {
  Ok(Json(page_review(&state.db, id).await?))
}

/// Sets a page's owner and review date. Saved at once: it is bookkeeping for the team, not content, and is never published.
#[utoipa::path(put, path = "/api/pages/{id}/review", operation_id = "setPageReview", params(("id" = PageId, Path)), request_body = PageReview, responses(
  (status = 200, body = PageReview),
  (status = 404, body = ErrorBody, description = "no such page, or it is in the trash"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn set_review(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>, Json(body): Json<PageReview>) -> Result<Json<PageReview>, AppError> {
  Ok(Json(set_page_review(&state.db, id, &body, user).await?))
}

/// Everyone who can own a page: the people who signed in, by name.
#[utoipa::path(get, path = "/api/users", operation_id = "listUsers", responses(
  (status = 200, body = Vec<User>),
))]
#[instrument(skip_all)]
async fn users(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Result<Json<Vec<User>>, AppError> {
  let mut users = list_users(&state.db).await?;
  // `dev` has no login and no row for its one user.
  if user == local_user_id() {
    users.insert(0, crate::me::local_user());
  }

  Ok(Json(users))
}

/// The pages whose review date has come, the most overdue first.
#[utoipa::path(get, path = "/api/reports/stale", operation_id = "stalePages", responses(
  (status = 200, body = Vec<StalePage>),
))]
#[instrument(skip_all)]
async fn stale(State(state): State<AppState>) -> Result<Json<Vec<StalePage>>, AppError> {
  Ok(Json(due(&state).await?))
}

/// The pages due for review today, in the server's time zone.
pub(crate) async fn due(state: &AppState) -> Result<Vec<StalePage>, AppError> {
  let mut due = Vec::new();
  for row in stale_pages(&state.db, Zoned::now().date()).await? {
    let owner = match row.owner_id {
      Some(id) if id == local_user_id() => Some(crate::me::local_user()),
      // An owner whose account is gone owns nothing.
      Some(id) => match get_user(&state.db, id).await {
        Ok(user) => Some(user),
        Err(cairn_store::StoreError::NotFound) => None,
        Err(error) => return Err(error.into()),
      },
      None => None,
    };
    due.push(StalePage {
      page_id: row.page_id,
      title: row.title,
      owner,
      review_by: row.review_by,
    });
  }

  Ok(due)
}
