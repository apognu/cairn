use axum::{
  Json,
  extract::{Query, State},
  http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_core::{Inactive, PageId, Redirect, RedirectOrigin, resolve_redirect};
use yourdocs_store::RedirectTarget;

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(list, add, remove))
}

/// A redirect as the Settings page shows it.
#[derive(Serialize, ToSchema)]
struct RedirectEntry {
  from_path: String,
  origin: RedirectOrigin,
  /// Title of the page it leads to, when it leads to a page.
  to_page_title: Option<String>,
  /// Where it leads now, after following chains; absent when it is not written.
  to: Option<String>,
  /// Why it is not written to the site, when it is not.
  inactive: Option<Inactive>,
}

#[derive(Deserialize, ToSchema)]
struct NewRedirect {
  from_path: String,
  /// Lead to this page...
  to_page_id: Option<PageId>,
  /// ...or to this URL or path.
  to_url: Option<String>,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct RedirectPath {
  path: String,
}

async fn entries(state: &AppState) -> Result<Vec<RedirectEntry>, AppError> {
  let pages = yourdocs_store::list_pages(&state.db).await?;
  let redirects: Vec<Redirect> = yourdocs_store::list_redirects(&state.db).await?;

  Ok(
    redirects
      .iter()
      .map(|r| {
        let resolved = resolve_redirect(r, &redirects, &pages);
        RedirectEntry {
          from_path: r.from_path.clone(),
          origin: r.origin,
          to_page_title: r.to_page_id.and_then(|id| pages.iter().find(|p| p.id == id)).map(|p| p.title.clone()),
          to: resolved.clone().ok(),
          inactive: resolved.err(),
        }
      })
      .collect(),
  )
}

/// Every redirect of the site, with where it leads and whether it is written.
#[utoipa::path(get, path = "/api/redirects", operation_id = "listRedirects", responses((status = 200, body = Vec<RedirectEntry>)))]
#[instrument(skip_all)]
async fn list(State(state): State<AppState>) -> Result<Json<Vec<RedirectEntry>>, AppError> {
  Ok(Json(entries(&state).await?))
}

/// Adds a redirect by hand.
#[utoipa::path(post, path = "/api/redirects", operation_id = "addRedirect", request_body = NewRedirect, responses(
  (status = 201, body = Vec<RedirectEntry>),
  (status = 409, body = ErrorBody, description = "the path belongs to a page or another redirect; the message names it"),
  (status = 422, body = ErrorBody, description = "not a usable path or target, or it would make a loop"),
))]
#[instrument(skip_all)]
async fn add(State(state): State<AppState>, Json(body): Json<NewRedirect>) -> Result<(StatusCode, Json<Vec<RedirectEntry>>), AppError> {
  let target = match (body.to_page_id, body.to_url) {
    (Some(id), None) => RedirectTarget::Page(id),
    (None, Some(url)) => RedirectTarget::Url(url),
    _ => return Err(AppError::Invalid(vec!["give either a page or a URL to redirect to".into()])),
  };
  yourdocs_store::add_redirect(&state.db, &body.from_path, target).await?;

  Ok((StatusCode::CREATED, Json(entries(&state).await?)))
}

/// Deletes a redirect.
#[utoipa::path(delete, path = "/api/redirects", operation_id = "removeRedirect", params(RedirectPath), responses(
  (status = 200, body = Vec<RedirectEntry>),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn remove(State(state): State<AppState>, Query(query): Query<RedirectPath>) -> Result<Json<Vec<RedirectEntry>>, AppError> {
  yourdocs_store::remove_redirect(&state.db, &query.path).await?;

  Ok(Json(entries(&state).await?))
}
