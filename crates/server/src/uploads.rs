use axum::{
  Json,
  body::Body,
  extract::{Path, State},
  http::{HeaderMap, HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
};
use cairn_assets::process;
use cairn_auth::CurrentUser;
use cairn_core::{Asset, AssetId, AssetStore, AssetVariant, CoreError, original_key, variant_key};
use cairn_store::{NewAsset, asset_by_hash, asset_by_id, get_or_create_asset};
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

/// Largest accepted upload. The body streams into the asset store, so this bounds the file decoded for its variants, not the transfer.
const MAX_UPLOAD: u64 = 25 * 1024 * 1024;

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new()
    .routes(routes!(upload))
    .layer(axum::middleware::from_fn(|request, next| crate::error::too_large_upload(MAX_UPLOAD / (1024 * 1024), request, next)))
}

/// Uploads an image (the request body is the file). Uploading bytes that are already stored returns the existing asset.
#[utoipa::path(post, path = "/api/assets", operation_id = "uploadAsset", request_body(content = String, content_type = "application/octet-stream"), responses(
  (status = 201, body = Asset, description = "stored as a new asset"),
  (status = 200, body = Asset, description = "the same file was already stored"),
  (status = 413, body = ErrorBody, description = "the file is too large"),
  (status = 422, body = ErrorBody, description = "not a supported image, too many pixels, or damaged"),
))]
#[instrument(skip_all)]
async fn upload(State(state): State<AppState>, CurrentUser(user): CurrentUser, headers: HeaderMap, body: Body) -> Result<(StatusCode, Json<Asset>), AppError> {
  let announced = headers.get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()?.parse::<u64>().ok());
  if announced.is_some_and(|length| length > MAX_UPLOAD) {
    return Err(CoreError::TooLarge { limit: MAX_UPLOAD }.into());
  }

  // The type and size come from the bytes, never from what the client says; the size is checked before any pixel is decoded.
  let staged = state.assets.stage(body.into_data_stream(), MAX_UPLOAD).await?;
  let key = original_key(&staged.sha256);

  if let Some(existing) = asset_by_hash(&state.db, &staged.sha256).await? {
    // The row exists but its file may not (a store emptied or switched): this upload puts it back.
    if state.assets.exists(&key).await? {
      state.assets.discard(staged).await?;
    } else {
      state.assets.keep(staged, &key).await?;
    }
    return Ok((StatusCode::OK, Json(existing)));
  }

  // Decoding and resizing are CPU heavy: never on the async threads.
  let source = state.assets.read_staged(&staged).await?;
  let details = staged.info.clone();
  let processed = match tokio::task::spawn_blocking(move || process(&source, &details)).await.map_err(|e| CoreError::Other(e.into()))? {
    Ok(processed) => processed,
    Err(error) => {
      state.assets.discard(staged).await?;
      return Err(error.into());
    }
  };

  // The files first, the row second: a row never points at a missing file, and an orphaned file is harmless.
  let (sha256, bytes, mime) = (staged.sha256.clone(), staged.bytes, staged.info.mime);
  state.assets.keep(staged, &key).await?;

  let mut variants = Vec::with_capacity(processed.variants.len());
  for variant in processed.variants {
    state.assets.put(&variant_key(&sha256, variant.width), variant.bytes.clone()).await?;
    variants.push(AssetVariant {
      width: variant.width,
      height: variant.height,
      bytes: variant.bytes.len() as u64,
    });
  }

  let (asset, created) = get_or_create_asset(
    &state.db,
    NewAsset {
      sha256,
      mime: mime.into(),
      width: Some(processed.width),
      height: Some(processed.height),
      bytes,
      variants,
      placeholder: processed.placeholder,
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
  let bytes = state.assets.get(&original_key(&asset.sha256)).await?;

  Ok(
    (
      [
        (header::CONTENT_TYPE, HeaderValue::from_str(&asset.mime).map_err(|e| CoreError::Other(e.into()))?),
        // The file behind an ID never changes, but it may not be published yet: a shared cache must not keep it.
        (header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=31536000, immutable")),
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

  use cairn_auth::local_user_id;

  use super::*;
  use crate::app::test_state;

  /// A complete 1x1 PNG: the upload decodes it to make its variants.
  const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
  ];

  #[tokio::test]
  async fn uploading_the_same_file_twice_stores_it_once() {
    let (state, dir) = crate::app::test_state_with_dir().await;
    let send = || upload(State(state.clone()), CurrentUser(local_user_id()), HeaderMap::new(), Body::from(PNG));

    let (first_status, Json(first)) = send().await.unwrap();
    let (second_status, Json(second)) = send().await.unwrap();

    assert_eq!((first_status, second_status), (StatusCode::CREATED, StatusCode::OK));
    assert_eq!(first.id, second.id);
    assert_eq!((first.width, first.height, first.mime.as_str()), (Some(1), Some(1), "image/png"));

    // The original and its one WebP variant, stored once however many times the file is uploaded.
    let files: Vec<_> = walk(&dir.join("assets"));
    assert_eq!(files.len(), 2, "{files:?}");
    assert_eq!(first.variants.len(), 1);
    assert!(first.placeholder.as_deref().is_some_and(|c| c.starts_with('#')));

    let response = serve(State(state), Path(format!("{}.png", first.id))).await.unwrap();
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
  }

  #[tokio::test]
  async fn rejects_files_that_are_not_images() {
    let state = test_state().await;
    let result = upload(State(state), CurrentUser(local_user_id()), HeaderMap::new(), Body::from(&b"<svg onload=alert(1)>"[..])).await;

    assert!(matches!(result, Err(AppError::Core(CoreError::Invalid(_)))));
  }

  #[tokio::test]
  async fn a_pixel_bomb_is_rejected_without_being_decoded() {
    let state = test_state().await;
    // The header of a 30000 x 30000 image: a few dozen bytes that would need gigabytes of memory to decode.
    let mut bomb = PNG.to_vec();
    bomb[16..20].copy_from_slice(&30_000u32.to_be_bytes());
    bomb[20..24].copy_from_slice(&30_000u32.to_be_bytes());

    let result = upload(State(state), CurrentUser(local_user_id()), HeaderMap::new(), Body::from(bomb)).await;

    assert!(matches!(result, Err(AppError::Core(CoreError::Invalid(m))) if m.contains("pixels")));
  }

  #[tokio::test]
  async fn a_file_past_the_limit_is_refused_while_it_streams_and_leaves_nothing() {
    let (state, dir) = crate::app::test_state_with_dir().await;
    // An image header followed by more than the limit, sent in chunks and without a length, as a client streaming it would.
    let chunks = std::iter::once(Ok::<_, std::io::Error>(bytes::Bytes::from_static(PNG))).chain((0..30).map(|_| Ok(bytes::Bytes::from(vec![0u8; 1 << 20]))));
    let body = Body::from_stream(futures_util::stream::iter(chunks));

    let result = upload(State(state), CurrentUser(local_user_id()), HeaderMap::new(), body).await;

    assert!(matches!(result, Err(AppError::Core(CoreError::TooLarge { .. }))));
    assert!(walk(&dir.join("assets")).is_empty());
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
