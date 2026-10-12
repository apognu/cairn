use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, sea_query::OnConflict};
use yourdocs_core::{PageId, Release, ReleaseId, ReleaseStatus, RevisionId, UserId};

use crate::{
  StoreError,
  entity::{release, release_page, setting},
};

/// Starts a release. `tree_snapshot` and `redirects_snapshot` are JSON, frozen with the release so it can be rebuilt or rolled back later.
pub async fn create_release(db: &DatabaseConnection, user: UserId, tree_snapshot: String, redirects_snapshot: String) -> Result<Release, StoreError> {
  let base = latest_live_release(db).await?;

  let row = release::ActiveModel {
    id: Set(ReleaseId::new().as_uuid()),
    base_release_id: Set(base.map(|r| r.id.as_uuid())),
    created_by: Set(user.as_uuid()),
    created_at: Set(Timestamp::now().to_string()),
    tree_snapshot: Set(tree_snapshot),
    redirects_snapshot: Set(redirects_snapshot),
    status: Set(ReleaseStatus::Building.as_str().into()),
    target_version_id: Set(None),
  }
  .insert(db)
  .await?;

  into_release(db, row).await
}

pub async fn set_release_status(db: &DatabaseConnection, id: ReleaseId, status: ReleaseStatus) -> Result<(), StoreError> {
  release::Entity::update_many()
    .col_expr(release::Column::Status, status.as_str().into())
    .filter(release::Column::Id.eq(id.as_uuid()))
    .exec(db)
    .await?;

  Ok(())
}

/// Which revision of each page went into a release, and the hash of the HTML built from it.
pub async fn add_release_pages(db: &DatabaseConnection, id: ReleaseId, pages: Vec<(PageId, RevisionId, String)>) -> Result<(), StoreError> {
  if pages.is_empty() {
    return Ok(());
  }

  let rows = pages.into_iter().map(|(page, revision, hash)| release_page::ActiveModel {
    release_id: Set(id.as_uuid()),
    page_id: Set(page.as_uuid()),
    revision_id: Set(revision.as_uuid()),
    output_hash: Set(Some(hash)),
  });
  release_page::Entity::insert_many(rows).exec(db).await?;

  Ok(())
}

pub async fn latest_live_release(db: &DatabaseConnection) -> Result<Option<Release>, StoreError> {
  let row = release::Entity::find()
    .filter(release::Column::Status.eq(ReleaseStatus::Live.as_str()))
    .order_by_desc(release::Column::Id)
    .one(db)
    .await?;

  match row {
    Some(row) => Ok(Some(into_release(db, row).await?)),
    None => Ok(None),
  }
}

async fn into_release(db: &DatabaseConnection, row: release::Model) -> Result<Release, StoreError> {
  let page_count = release_page::Entity::find().filter(release_page::Column::ReleaseId.eq(row.id)).count(db).await?;

  Ok(Release {
    id: row.id.into(),
    base_release_id: row.base_release_id.map(Into::into),
    created_by: row.created_by.into(),
    created_at: row.created_at.parse().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")))?,
    status: ReleaseStatus::parse(&row.status).ok_or_else(|| StoreError::Invalid(format!("unknown release status {}", row.status)))?,
    page_count: page_count as u32,
  })
}

pub async fn get_setting(db: &DatabaseConnection, key: &str) -> Result<Option<String>, StoreError> {
  Ok(setting::Entity::find_by_id(key).one(db).await?.map(|s| s.value))
}

pub async fn set_setting(db: &DatabaseConnection, key: &str, value: &str) -> Result<(), StoreError> {
  setting::Entity::insert(setting::ActiveModel {
    key: Set(key.into()),
    value: Set(value.into()),
  })
  .on_conflict(OnConflict::column(setting::Column::Key).update_column(setting::Column::Value).to_owned())
  .exec(db)
  .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::connect::test_db;

  #[tokio::test]
  async fn releases_start_building_and_settings_overwrite() {
    let db = test_db().await;

    let release = create_release(&db, UserId::new(), "[]".into(), "[]".into()).await.unwrap();
    assert_eq!(release.status, ReleaseStatus::Building);
    assert!(latest_live_release(&db).await.unwrap().is_none());

    set_release_status(&db, release.id, ReleaseStatus::Live).await.unwrap();
    let live = latest_live_release(&db).await.unwrap().unwrap();
    assert_eq!(live.id, release.id);
    assert_eq!(create_release(&db, UserId::new(), "[]".into(), "[]".into()).await.unwrap().base_release_id, Some(release.id));

    assert_eq!(get_setting(&db, "site.name").await.unwrap(), None);
    set_setting(&db, "site.name", "A").await.unwrap();
    set_setting(&db, "site.name", "B").await.unwrap();
    assert_eq!(get_setting(&db, "site.name").await.unwrap().as_deref(), Some("B"));
  }
}
