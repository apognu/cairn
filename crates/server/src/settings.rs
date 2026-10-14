use std::collections::BTreeMap;

use axum::{Json, extract::State, http::header, response::IntoResponse};
use cairn_core::{CoreError, Mode, Settings};
use cairn_store::{StoreError, asset_by_id, load_settings, save_settings};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new()
    .routes(routes!(get_settings, put_settings))
    .routes(routes!(check_settings))
    .routes(routes!(theme_css))
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
  // A logo must be an asset that exists, or every page would publish a broken header image.
  for logo in [settings.logo_asset_id, settings.logo_dark_asset_id].into_iter().flatten() {
    asset_by_id(&state.db, logo).await.map_err(|e| match e {
      StoreError::NotFound => AppError::from(CoreError::Invalid("the logo is not an uploaded image".into())),
      e => e.into(),
    })?;
  }

  save_settings(&state.db, &settings).await?;

  Ok(Json(load_settings(&state.db).await?))
}

/// What the settings form shows while the palette is being edited.
#[derive(Serialize, ToSchema)]
pub(crate) struct ThemeCheck {
  /// Text pairs under WCAG AA; saving is refused while there are any.
  problems: Vec<String>,
  /// The custom properties each mode gets, for the live preview.
  light: BTreeMap<String, String>,
  dark: BTreeMap<String, String>,
}

/// Checks unsaved settings: the contrast problems and the custom properties of each mode.
#[utoipa::path(post, path = "/api/settings/check", operation_id = "checkSettings", request_body = Settings, responses(
  (status = 200, body = ThemeCheck),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn check_settings(Json(settings): Json<Settings>) -> Result<Json<ThemeCheck>, AppError> {
  settings.validate_shape().map_err(CoreError::Invalid)?;

  let vars = |mode| {
    let mut vars: BTreeMap<String, String> = settings.theme_vars(mode).into_iter().map(|(name, value)| (name.to_string(), value)).collect();
    vars.insert("--accent-base".into(), settings.accent.to_lowercase());
    vars
  };

  Ok(Json(ThemeCheck {
    problems: settings.contrast_problems(),
    light: vars(Mode::Light),
    dark: vars(Mode::Dark),
  }))
}

/// The saved theme as a stylesheet, which the editor loads after the site's so authors write inside the site's colors.
#[utoipa::path(get, path = "/api/theme.css", operation_id = "getThemeCss", responses((status = 200, content_type = "text/css", body = String)))]
#[instrument(skip_all)]
async fn theme_css(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
  let css = load_settings(&state.db).await?.theme_css();

  Ok(([(header::CONTENT_TYPE, "text/css; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")], css))
}
