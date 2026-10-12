use axum::{
  Json,
  body::Bytes,
  extract::{DefaultBodyLimit, Path, State},
  http::{HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
};
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_assets::{inspect, sha256_hex};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{Asset, AssetId, AssetStore, CoreError};
use yourdocs_store::{NewAsset, asset_by_id, get_or_create_asset};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

/// Largest accepted upload. The body is held in memory for now; streaming comes with the production hardening (M7).
const MAX_UPLOAD: usize = 25 * 1024 * 1024;

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(upload)).layer(DefaultBodyLimit::max(MAX_UPLOAD))
}

fn key(sha256: &str) -> String {
  format!("originals/{sha256}")
}

/// Uploads an image (the request body is the file). Uploading bytes that are already stored returns the existing asset.
#[utoipa::path(post, path = "/api/assets", operation_id = "uploadAsset", request_body(content = String, content_type = "application/octet-stream"), responses(
  (status = 201, body = Asset, description = "stored as a new asset"),
  (status = 200, body = Asset, description = "the same file was already stored"),
  (status = 413, description = "the file is too large"),
  (status = 422, body = ErrorBody, description = "not a supported image"),
))]
#[instrument(skip_all)]
async fn upload(State(state): State<AppState>, CurrentUser(user): CurrentUser, body: Bytes) -> Result<(StatusCode, Json<Asset>), AppError> {
  // The type comes from the bytes, never from what the client says it is.
  let info = inspect(&body)?;
  let bytes = body.len() as u64;

  let (sha256, body) = tokio::task::spawn_blocking(move || (sha256_hex(&body), body)).await.map_err(|e| CoreError::Other(e.into()))?;

  // The file first, the row second: a row never points at a missing file, and an orphaned file is harmless.
  let key = key(&sha256);
  if !state.assets.exists(&key).await? {
    state.assets.put(&key, body.to_vec()).await?;
  }

  let (asset, created) = get_or_create_asset(
    &state.db,
    NewAsset {
      sha256,
      mime: info.mime.into(),
      width: Some(info.width),
      height: Some(info.height),
      bytes,
      uploaded_by: user,
    },
  )
  .await?;

  Ok((if created { StatusCode::CREATED } else { StatusCode::OK }, Json(asset)))
}

/// The original file of an asset, for the editor. `/_a/<id>` and `/_a/<id>.<ext>` both work.
#[instrument(skip_all, fields(asset = %name))]
pub(crate) async fn serve(State(state): State<AppState>, Path(name): Path<String>) -> Result<Response, AppError> {
  let id: AssetId = name.split('.').next().unwrap_or_default().parse().map_err(|_| AppError::Core(CoreError::NotFound))?;
  let asset = asset_by_id(&state.db, id).await?;
  let bytes = state.assets.get(&key(&asset.sha256)).await?;

  Ok(
    (
      [
        (header::CONTENT_TYPE, HeaderValue::from_str(&asset.mime).map_err(|e| CoreError::Other(e.into()))?),
        // The file behind an ID never changes.
        (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable")),
        // Uploads are untrusted: never let a browser treat one as anything but the image it was sniffed as.
        (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
      ],
      bytes,
    )
      .into_response(),
  )
}

#[cfg(test)]
mod tests {
  use std::fs;

  use yourdocs_auth::local_user_id;

  use super::*;
  use crate::app::test_state;

  const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89,
  ];

  #[tokio::test]
  async fn uploading_the_same_file_twice_stores_it_once() {
    let (state, dir) = crate::app::test_state_with_dir().await;
    let send = || upload(State(state.clone()), CurrentUser(local_user_id()), Bytes::from_static(PNG));

    let (first_status, Json(first)) = send().await.unwrap();
    let (second_status, Json(second)) = send().await.unwrap();

    assert_eq!((first_status, second_status), (StatusCode::CREATED, StatusCode::OK));
    assert_eq!(first.id, second.id);
    assert_eq!((first.width, first.height, first.mime.as_str()), (Some(1), Some(1), "image/png"));

    let files: Vec<_> = walk(&dir.join("assets"));
    assert_eq!(files.len(), 1, "{files:?}");

    let response = serve(State(state), Path(format!("{}.png", first.id))).await.unwrap();
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
  }

  #[tokio::test]
  async fn rejects_files_that_are_not_images() {
    let state = test_state().await;
    let result = upload(State(state), CurrentUser(local_user_id()), Bytes::from_static(b"<svg onload=alert(1)>")).await;

    assert!(matches!(result, Err(AppError::Core(CoreError::Invalid(_)))));
  }

  fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
      let path = entry.path();
      if path.is_dir() {
        found.extend(walk(&path));
      } else {
        found.push(path);
      }
    }
    found
  }
}
