use axum::{
  Json, Router,
  extract::{Path, State},
  http::StatusCode,
  routing::{get, post},
};
use serde::Deserialize;
use tracing::instrument;
use yourdocs_auth::CurrentUser;
use yourdocs_core::{Page, PageId, Placement};

use crate::{app::AppState, error::AppError};

pub(crate) fn routes() -> Router<AppState> {
  Router::new()
    .route("/api/pages", get(list).post(create))
    .route("/api/pages/{id}", post(rename).delete(remove))
    .route("/api/pages/{id}/move", post(move_to))
}

#[derive(Deserialize)]
struct CreatePage {
  parent_id: Option<PageId>,
  title: String,
  /// Insert before this sibling; the page goes last when absent.
  before: Option<PageId>,
}

#[derive(Deserialize)]
struct RenamePage {
  title: String,
}

#[derive(Deserialize)]
struct MovePage {
  parent_id: Option<PageId>,
  before: Option<PageId>,
}

fn placement(before: Option<PageId>) -> Placement {
  before.map_or(Placement::Last, Placement::Before)
}

#[instrument(skip_all)]
async fn list(State(state): State<AppState>) -> Result<Json<Vec<Page>>, AppError> {
  Ok(Json(yourdocs_store::list_pages(&state.db).await?))
}

#[instrument(skip_all)]
async fn create(State(state): State<AppState>, CurrentUser(user): CurrentUser, Json(body): Json<CreatePage>) -> Result<(StatusCode, Json<Page>), AppError> {
  let page = yourdocs_store::create_page(&state.db, user, body.parent_id, &body.title, placement(body.before)).await?;

  Ok((StatusCode::CREATED, Json(page)))
}

#[instrument(skip_all, fields(page_id = %id))]
async fn rename(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<RenamePage>) -> Result<Json<Page>, AppError> {
  Ok(Json(yourdocs_store::rename_page(&state.db, id, &body.title).await?))
}

#[instrument(skip_all, fields(page_id = %id))]
async fn move_to(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<MovePage>) -> Result<Json<Page>, AppError> {
  Ok(Json(yourdocs_store::move_page(&state.db, id, body.parent_id, placement(body.before)).await?))
}

#[instrument(skip_all, fields(page_id = %id))]
async fn remove(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<StatusCode, AppError> {
  yourdocs_store::delete_page(&state.db, id).await?;

  Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
  use yourdocs_auth::local_user_id;

  use super::*;
  use crate::app::test_state;

  #[tokio::test]
  async fn writes_record_the_local_user() {
    let state = test_state().await;
    let body = CreatePage {
      parent_id: None,
      title: "Intro".into(),
      before: None,
    };

    let (status, Json(page)) = create(State(state), CurrentUser(local_user_id()), Json(body)).await.unwrap();

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(page.created_by, local_user_id());
  }
}
