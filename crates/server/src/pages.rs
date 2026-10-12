use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
};
use serde::Deserialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{OldUrls, Page, PageId, Placement};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new()
    .routes(routes!(list, create))
    .routes(routes!(rename, remove))
    .routes(routes!(move_to))
    .routes(routes!(header))
    .routes(routes!(describe))
}

#[derive(Deserialize, ToSchema)]
struct CreatePage {
  parent_id: Option<PageId>,
  title: String,
  /// Insert before this sibling; the page goes last when absent.
  before: Option<PageId>,
  /// Makes the new node an external link to this URL instead of a page.
  external_url: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct RenamePage {
  title: String,
  /// Keep the old URL working with a redirect. Defaults to true.
  redirect: Option<bool>,
  /// The new target of a link; refused for pages.
  external_url: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct MovePage {
  parent_id: Option<PageId>,
  before: Option<PageId>,
  /// Keep the old URLs of the page and its subtree working with redirects. Defaults to true.
  redirect: Option<bool>,
}

fn old_urls(redirect: Option<bool>) -> OldUrls {
  if redirect.unwrap_or(true) { OldUrls::Redirect } else { OldUrls::Forget }
}

fn placement(before: Option<PageId>) -> Placement {
  before.map_or(Placement::Last, Placement::Before)
}

/// Every live page, ordered by position.
#[utoipa::path(get, path = "/api/pages", operation_id = "listPages", responses((status = 200, body = Vec<Page>)))]
#[instrument(skip_all)]
async fn list(State(state): State<AppState>) -> Result<Json<Vec<Page>>, AppError> {
  Ok(Json(yourdocs_store::list_pages(&state.db).await?))
}

/// Creates a page, or an external link when `external_url` is set, under `parent_id` (or at the root).
#[utoipa::path(post, path = "/api/pages", operation_id = "createPage", request_body = CreatePage, responses(
  (status = 201, body = Page),
  (status = 404, body = ErrorBody, description = "unknown parent"),
  (status = 409, body = ErrorBody, description = "a sibling has the same URL segment"),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn create(State(state): State<AppState>, CurrentUser(user): CurrentUser, Json(body): Json<CreatePage>) -> Result<(StatusCode, Json<Page>), AppError> {
  let place = placement(body.before);
  let page = match &body.external_url {
    Some(url) => yourdocs_store::create_link(&state.db, user, body.parent_id, &body.title, url, place).await?,
    None => yourdocs_store::create_page(&state.db, user, body.parent_id, &body.title, place).await?,
  };

  Ok((StatusCode::CREATED, Json(page)))
}

/// Changes the title (the URL segment follows it) and, for a link, its target.
#[utoipa::path(post, path = "/api/pages/{id}", operation_id = "renamePage", params(("id" = PageId, Path)), request_body = RenamePage, responses(
  (status = 200, body = Page),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "a sibling has the same URL segment"),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn rename(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<RenamePage>) -> Result<Json<Page>, AppError> {
  let page = match &body.external_url {
    Some(url) => yourdocs_store::edit_link(&state.db, id, &body.title, url).await?,
    None => yourdocs_store::rename_page(&state.db, id, &body.title, old_urls(body.redirect)).await?,
  };

  Ok(Json(page))
}

/// Moves a page and its subtree under another parent.
#[utoipa::path(post, path = "/api/pages/{id}/move", operation_id = "movePage", params(("id" = PageId, Path)), request_body = MovePage, responses(
  (status = 200, body = Page),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "a sibling has the same URL segment"),
  (status = 422, body = ErrorBody, description = "the target is inside the page's own subtree"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn move_to(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<MovePage>) -> Result<Json<Page>, AppError> {
  Ok(Json(yourdocs_store::move_page(&state.db, id, body.parent_id, placement(body.before), old_urls(body.redirect)).await?))
}

#[derive(Deserialize, ToSchema)]
struct HeaderBody {
  in_header: bool,
}

/// Shows the page in the site header, or takes it out. A promoted page with subpages gets a sidebar of its own.
#[utoipa::path(put, path = "/api/pages/{id}/header", operation_id = "setPageHeader", params(("id" = PageId, Path)), request_body = HeaderBody, responses(
  (status = 200, body = Page),
  (status = 404, body = ErrorBody),
  (status = 422, body = ErrorBody, description = "links cannot be shown in the header"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn header(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<HeaderBody>) -> Result<Json<Page>, AppError> {
  Ok(Json(yourdocs_store::set_in_header(&state.db, id, body.in_header).await?))
}

#[derive(Deserialize, ToSchema)]
struct DescriptionBody {
  /// Up to 300 characters; empty clears it.
  description: String,
}

/// Sets the page's short description, shown on link cards and in its meta tags.
#[utoipa::path(put, path = "/api/pages/{id}/description", operation_id = "setPageDescription", params(("id" = PageId, Path)), request_body = DescriptionBody, responses(
  (status = 200, body = Page),
  (status = 404, body = ErrorBody),
  (status = 422, body = ErrorBody, description = "too long"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn describe(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<DescriptionBody>) -> Result<Json<Page>, AppError> {
  Ok(Json(yourdocs_store::set_description(&state.db, id, &body.description).await?))
}

/// Deletes a page and its descendants.
#[utoipa::path(delete, path = "/api/pages/{id}", operation_id = "deletePage", params(("id" = PageId, Path)), responses((status = 204), (status = 404, body = ErrorBody)))]
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
      external_url: None,
    };

    let (status, Json(page)) = create(State(state), CurrentUser(local_user_id()), Json(body)).await.unwrap();

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(page.created_by, local_user_id());
  }
}
