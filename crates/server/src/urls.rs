use axum::{
  Json,
  extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_core::{OldUrls, PageId, RedirectOrigin, page_urls};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(get_url, set_url)).routes(routes!(add_alias, remove_alias))
}

/// Where a page lives and which old URLs lead to it.
#[derive(Serialize, ToSchema)]
struct PageUrl {
  /// The URL the page has now.
  url: String,
  /// The URL it would have without a custom one.
  derived_url: String,
  custom_url: Option<String>,
  aliases: Vec<Alias>,
}

#[derive(Serialize, ToSchema)]
struct Alias {
  path: String,
  origin: RedirectOrigin,
  #[schema(value_type = String, format = DateTime)]
  created_at: jiff::Timestamp,
}

#[derive(Deserialize, ToSchema)]
struct SetUrl {
  /// The fixed URL, or null to use the one derived from the tree.
  custom_url: Option<String>,
  /// Keep the old URLs of the page and the pages below it as redirects. Defaults to true.
  redirect: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
struct NewAlias {
  path: String,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct AliasPath {
  path: String,
}

async fn describe(state: &AppState, id: PageId) -> Result<PageUrl, AppError> {
  let mut pages = yourdocs_store::list_pages(&state.db).await?;
  let page = pages.iter().find(|p| p.id == id).ok_or(AppError::NotFound)?;
  let custom_url = page.path_override.clone();
  let url = page_urls(&pages).get(&id).cloned().ok_or(AppError::NotFound)?;

  if let Some(page) = pages.iter_mut().find(|p| p.id == id) {
    page.path_override = None;
  }
  let derived_url = page_urls(&pages).get(&id).cloned().unwrap_or_else(|| url.clone());

  let aliases = yourdocs_store::list_aliases(&state.db, id)
    .await?
    .into_iter()
    .map(|r| Alias {
      path: r.from_path,
      origin: r.origin,
      created_at: r.created_at,
    })
    .collect();

  Ok(PageUrl {
    url,
    derived_url,
    custom_url,
    aliases,
  })
}

/// The page's URL, custom URL and aliases.
#[utoipa::path(get, path = "/api/pages/{id}/url", operation_id = "getPageUrl", params(("id" = PageId, Path)), responses(
  (status = 200, body = PageUrl),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_url(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<PageUrl>, AppError> {
  Ok(Json(describe(&state, id).await?))
}

/// Sets or clears the page's custom URL.
#[utoipa::path(put, path = "/api/pages/{id}/url", operation_id = "setPageUrl", params(("id" = PageId, Path)), request_body = SetUrl, responses(
  (status = 200, body = PageUrl),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "the URL belongs to another page or alias; the message names it"),
  (status = 422, body = ErrorBody, description = "not a usable path"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn set_url(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<SetUrl>) -> Result<Json<PageUrl>, AppError> {
  let old = if body.redirect.unwrap_or(true) { OldUrls::Redirect } else { OldUrls::Forget };
  yourdocs_store::set_custom_url(&state.db, id, body.custom_url.as_deref(), old).await?;

  Ok(Json(describe(&state, id).await?))
}

/// Adds an alias: an old URL that redirects to the page.
#[utoipa::path(post, path = "/api/pages/{id}/aliases", operation_id = "addAlias", params(("id" = PageId, Path)), request_body = NewAlias, responses(
  (status = 200, body = PageUrl),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "the path belongs to another page or alias; the message names it"),
  (status = 422, body = ErrorBody, description = "not a usable path"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn add_alias(State(state): State<AppState>, Path(id): Path<PageId>, Json(body): Json<NewAlias>) -> Result<Json<PageUrl>, AppError> {
  yourdocs_store::add_alias(&state.db, id, &body.path).await?;

  Ok(Json(describe(&state, id).await?))
}

/// Removes an alias of the page.
#[utoipa::path(delete, path = "/api/pages/{id}/aliases", operation_id = "removeAlias", params(("id" = PageId, Path), AliasPath), responses(
  (status = 200, body = PageUrl),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn remove_alias(State(state): State<AppState>, Path(id): Path<PageId>, Query(query): Query<AliasPath>) -> Result<Json<PageUrl>, AppError> {
  yourdocs_store::remove_alias(&state.db, id, &query.path).await?;

  Ok(Json(describe(&state, id).await?))
}
