use axum::{
  Json,
  extract::{Path, State},
};
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{ApiConfig, PageId};
use yourdocs_store::{get_api_config, save_api_config};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(get_config, put_config))
}

/// The sources an API reference page is built from.
#[utoipa::path(get, path = "/api/pages/{id}/api-config", operation_id = "getApiConfig", params(("id" = PageId, Path)), responses(
  (status = 200, body = ApiConfig),
  (status = 404, body = ErrorBody),
  (status = 422, body = ErrorBody, description = "not an API reference page"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_config(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<ApiConfig>, AppError> {
  Ok(Json(get_api_config(&state.db, id).await?))
}

/// Saves the sources. The specs themselves are fetched at every publish, so nothing here counts as a pending change.
#[utoipa::path(put, path = "/api/pages/{id}/api-config", operation_id = "saveApiConfig", params(("id" = PageId, Path)), request_body = ApiConfig, responses(
  (status = 200, body = ApiConfig),
  (status = 404, body = ErrorBody),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn put_config(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>, Json(config): Json<ApiConfig>) -> Result<Json<ApiConfig>, AppError> {
  save_api_config(&state.db, id, &config, user).await?;

  Ok(Json(get_api_config(&state.db, id).await?))
}
