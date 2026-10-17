//! Page owners, review dates and the report of pages due for a review.

use axum::{
  Json,
  extract::{Path, State},
};
use cairn_auth::{Editor, local_user_id};
use cairn_core::{PageId, PageReview, StalePage};
use cairn_store::{StoreError, get_user, mark_reviewed, page_review, set_page_review, stale_pages};
use jiff::{Zoned, civil::Date};
use serde::Deserialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(get_review, set_review)).routes(routes!(reviewed)).routes(routes!(stale))
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
async fn set_review(State(state): State<AppState>, Editor(user): Editor, Path(id): Path<PageId>, Json(body): Json<PageReview>) -> Result<Json<PageReview>, AppError> {
  Ok(Json(set_page_review(&state.db, id, &body, user).await?))
}

#[derive(Deserialize, ToSchema)]
struct Reviewed {
  /// When the page should be reviewed again, after today.
  #[schema(value_type = String, format = Date)]
  next_review_by: Date,
}

/// Records that the signed-in user reviewed the page, and sets its next review date. It leaves the report of pages to review.
#[utoipa::path(post, path = "/api/pages/{id}/review/done", operation_id = "markPageReviewed", params(("id" = PageId, Path)), request_body = Reviewed, responses(
  (status = 200, body = PageReview),
  (status = 404, body = ErrorBody, description = "no such page, or it is in the trash"),
  (status = 422, body = ErrorBody, description = "the next review date is not after today"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn reviewed(State(state): State<AppState>, Editor(user): Editor, Path(id): Path<PageId>, Json(body): Json<Reviewed>) -> Result<Json<PageReview>, AppError> {
  if body.next_review_by <= Zoned::now().date() {
    return Err(StoreError::Invalid("the next review date must be after today".into()).into());
  }

  Ok(Json(mark_reviewed(&state.db, id, body.next_review_by, user).await?))
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
        Err(StoreError::NotFound) => None,
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
