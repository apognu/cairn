use axum::{Json, extract::State};
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_core::{CoreError, Settings};
use yourdocs_store::{StoreError, asset_by_id, load_settings, save_settings};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(get_settings, put_settings))
}

/// The site name, logo and accent color.
#[utoipa::path(get, path = "/api/settings", operation_id = "getSettings", responses((status = 200, body = Settings)))]
#[instrument(skip_all)]
async fn get_settings(State(state): State<AppState>) -> Result<Json<Settings>, AppError> {
  Ok(Json(load_settings(&state.db).await?))
}

/// Saves the settings. They reach the published site with the next publish.
#[utoipa::path(put, path = "/api/settings", operation_id = "saveSettings", request_body = Settings, responses(
  (status = 200, body = Settings),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn put_settings(State(state): State<AppState>, Json(settings): Json<Settings>) -> Result<Json<Settings>, AppError> {
  // The logo must be an asset that exists, or every page would publish a broken header image.
  if let Some(logo) = settings.logo_asset_id {
    asset_by_id(&state.db, logo).await.map_err(|e| match e {
      StoreError::NotFound => AppError::from(CoreError::Invalid("the logo is not an uploaded image".into())),
      e => e.into(),
    })?;
  }

  save_settings(&state.db, &settings).await?;

  Ok(Json(load_settings(&state.db).await?))
}
