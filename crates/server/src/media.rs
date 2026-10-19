//! The media library: uploaded images listed and searched, named, and taken out once nothing shows them.

use axum::{
  Json,
  extract::{Path, Query, State},
  http::StatusCode,
};
use cairn_auth::Editor;
use cairn_core::{Asset, AssetId, PageId};
use cairn_store::{StoreError, asset_by_id, asset_usage, get_user, list_assets, set_asset_title, set_kept};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

const PAGE: u64 = 48;
const MAX_PAGE: u64 = 200;

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(list)).routes(routes!(detail, rename, remove))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
  /// Part of the title or file name, ignoring case.
  q: Option<String>,
  /// The last asset of the previous page; the list goes on with older ones.
  before: Option<AssetId>,
  limit: Option<u64>,
}

/// Uploaded images, newest first, a page at a time.
#[utoipa::path(get, path = "/api/assets", operation_id = "listAssets", params(ListQuery), responses(
  (status = 200, body = Vec<Asset>),
))]
#[instrument(skip_all)]
async fn list(State(state): State<AppState>, Editor(_): Editor, Query(query): Query<ListQuery>) -> Result<Json<Vec<Asset>>, AppError> {
  let limit = query.limit.unwrap_or(PAGE).clamp(1, MAX_PAGE);

  Ok(Json(list_assets(&state.db, query.q.as_deref(), query.before, limit).await?))
}

#[derive(Serialize, ToSchema)]
struct UsedOn {
  id: PageId,
  title: String,
}

/// An asset and everything that still shows it.
#[derive(Serialize, ToSchema)]
struct AssetDetail {
  asset: Asset,
  /// Who uploaded it; none when that account is gone.
  uploaded_by_name: Option<String>,
  /// The image it was made from, when it is a copy.
  source: Option<Asset>,
  pages: Vec<UsedOn>,
  /// Keys of the library entries (snippets) that show it.
  entries: Vec<String>,
  /// The site settings use it (the logo).
  in_settings: bool,
  /// Kept copies made from it.
  copies: Vec<AssetId>,
}

/// One asset with where it is used.
#[utoipa::path(get, path = "/api/assets/{id}", operation_id = "getAsset", responses(
  (status = 200, body = AssetDetail),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn detail(State(state): State<AppState>, Editor(_): Editor, Path(id): Path<AssetId>) -> Result<Json<AssetDetail>, AppError> {
  let asset = asset_by_id(&state.db, id).await?;
  let usage = asset_usage(&state.db, id).await?;

  let uploaded_by_name = get_user(&state.db, asset.uploaded_by).await.ok().map(|user| user.name);
  let source = match asset.derived_from {
    Some(source) => asset_by_id(&state.db, source).await.ok(),
    None => None,
  };

  Ok(Json(AssetDetail {
    asset,
    uploaded_by_name,
    source,
    pages: usage.pages.into_iter().map(|(id, title)| UsedOn { id, title }).collect(),
    entries: usage.entries,
    in_settings: usage.settings,
    copies: usage.copies,
  }))
}

#[derive(Deserialize, ToSchema)]
struct Rename {
  /// Blank removes the title.
  title: String,
}

/// Names an asset, so it can be found in the media library.
#[utoipa::path(patch, path = "/api/assets/{id}", operation_id = "renameAsset", request_body = Rename, responses(
  (status = 200, body = Asset),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn rename(State(state): State<AppState>, Editor(_): Editor, Path(id): Path<AssetId>, Json(body): Json<Rename>) -> Result<Json<Asset>, AppError> {
  Ok(Json(set_asset_title(&state.db, id, &body.title).await?))
}

/// Takes an asset out of the media library. Its files go with the next cleanup after the grace period, so a page restored from an older
/// version in the meantime still shows it.
#[utoipa::path(delete, path = "/api/assets/{id}", operation_id = "deleteAsset", responses(
  (status = 204),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "a page, snippet, the settings or a kept copy still uses it"),
))]
#[instrument(skip_all)]
async fn remove(State(state): State<AppState>, Editor(_): Editor, Path(id): Path<AssetId>) -> Result<StatusCode, AppError> {
  asset_by_id(&state.db, id).await?;
  let usage = asset_usage(&state.db, id).await?;

  if !usage.is_empty() {
    let mut users: Vec<String> = usage.pages.into_iter().map(|(_, title)| format!("“{title}”")).collect();
    users.extend(usage.entries.into_iter().map(|key| format!("the snippet {key}")));
    if usage.settings {
      users.push("the site settings".into());
    }
    match usage.copies.len() {
      0 => {}
      1 => users.push("a copy made from it".into()),
      n => users.push(format!("{n} copies made from it")),
    }
    return Err(StoreError::Conflict(format!("This image is still used by {}.", users.join(", "))).into());
  }

  set_kept(&state.db, id, false).await?;
  Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
  use cairn_auth::local_user_id;
  use cairn_core::{ContentStore, Document, Placement};
  use cairn_store::{NewAsset, SqlContentStore, create_page, get_or_create_asset};

  use super::*;
  use crate::app::test_state;

  #[tokio::test]
  async fn an_image_still_shown_on_a_page_cannot_be_deleted() {
    let state = test_state().await;
    let user = local_user_id();
    let new = NewAsset {
      sha256: "0".repeat(64),
      mime: "image/png".into(),
      width: Some(1),
      height: Some(1),
      bytes: 1,
      variants: vec![],
      placeholder: None,
      uploaded_by: user,
      name: Some("shot.png".into()),
      title: None,
      derived_from: None,
      kept: true,
    };
    let (asset, _) = get_or_create_asset(&state.db, new).await.unwrap();
    let page = create_page(&state.db, user, None, "Install guide", Placement::Last).await.unwrap();
    let doc: Document = serde_json::from_value(serde_json::json!({ "type": "doc", "content": [{ "type": "image", "attrs": { "assetId": asset.id } }] })).unwrap();
    let content = SqlContentStore::new(state.db.clone());
    let base = content.save_draft(page.id, doc, None, user).await.unwrap().base_revision_id;

    let Err(AppError::Store(StoreError::Conflict(reason))) = remove(State(state.clone()), Editor(user), Path(asset.id)).await else {
      panic!("deleting an image in use is refused");
    };
    assert!(reason.contains("“Install guide”"), "{reason}");

    content.save_draft(page.id, Document::paragraph("no image"), base, user).await.unwrap();
    assert_eq!(remove(State(state.clone()), Editor(user), Path(asset.id)).await.unwrap(), StatusCode::NO_CONTENT);
    assert!(!asset_by_id(&state.db, asset.id).await.unwrap().kept);
  }
}
