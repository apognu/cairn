use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use yourdocs_core::{Asset, AssetId, UserId};

use crate::{StoreError, entity::asset};

/// What is known about a file before it has an ID.
pub struct NewAsset {
  pub sha256: String,
  pub mime: String,
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub bytes: u64,
  pub uploaded_by: UserId,
}

pub async fn asset_by_id(db: &DatabaseConnection, id: AssetId) -> Result<Asset, StoreError> {
  let row = asset::Entity::find_by_id(id.as_uuid()).one(db).await?;

  into_asset(row.ok_or(StoreError::NotFound)?)
}

/// Records an uploaded file, or returns the existing asset when the same bytes were uploaded before. The boolean says which happened.
pub async fn get_or_create_asset(db: &DatabaseConnection, new: NewAsset) -> Result<(Asset, bool), StoreError> {
  if let Some(existing) = by_hash(db, &new.sha256).await? {
    return Ok((existing, false));
  }

  let inserted = asset::ActiveModel {
    id: Set(AssetId::new().as_uuid()),
    sha256: Set(new.sha256.clone()),
    mime: Set(new.mime),
    width: Set(new.width.map(i64::from)),
    height: Set(new.height.map(i64::from)),
    bytes: Set(new.bytes as i64),
    variants: Set("[]".into()),
    placeholder: Set(None),
    uploaded_by: Set(new.uploaded_by.as_uuid()),
    created_at: Set(Timestamp::now().to_string()),
  }
  .insert(db)
  .await;

  match inserted {
    Ok(row) => Ok((into_asset(row)?, true)),
    // Two uploads of the same file raced and the other won the unique hash.
    Err(error) => match by_hash(db, &new.sha256).await? {
      Some(existing) => Ok((existing, false)),
      None => Err(error.into()),
    },
  }
}

async fn by_hash(db: &DatabaseConnection, sha256: &str) -> Result<Option<Asset>, StoreError> {
  let row = asset::Entity::find().filter(asset::Column::Sha256.eq(sha256)).one(db).await?;

  row.map(into_asset).transpose()
}

fn into_asset(row: asset::Model) -> Result<Asset, StoreError> {
  Ok(Asset {
    id: row.id.into(),
    sha256: row.sha256,
    mime: row.mime,
    width: row.width.map(|w| w as u32),
    height: row.height.map(|h| h as u32),
    bytes: row.bytes as u64,
    uploaded_by: row.uploaded_by.into(),
    created_at: row.created_at.parse().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")))?,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::connect::test_db;

  #[tokio::test]
  async fn the_same_hash_is_one_asset() {
    let db = test_db().await;
    let new = || NewAsset {
      sha256: "abc".into(),
      mime: "image/png".into(),
      width: Some(1),
      height: Some(1),
      bytes: 10,
      uploaded_by: UserId::new(),
    };

    let (first, created) = get_or_create_asset(&db, new()).await.unwrap();
    let (second, created_again) = get_or_create_asset(&db, new()).await.unwrap();

    assert!(created && !created_again);
    assert_eq!(first.id, second.id);
    assert_eq!(asset_by_id(&db, first.id).await.unwrap().mime, "image/png");
  }
}
