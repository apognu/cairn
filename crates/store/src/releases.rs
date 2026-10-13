use std::collections::HashMap;

use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, sea_query::OnConflict};
use yourdocs_core::{AssetId, Page, PageId, Release, ReleaseId, ReleaseStatus, RevisionId, Settings, UserId};

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

/// What the live release published: its tree and redirect rules as frozen at the time, and the revision of each page.
pub struct LiveRelease {
  pub pages: Vec<Page>,
  pub redirects: Vec<(String, String)>,
  pub revisions: HashMap<PageId, RevisionId>,
}

/// The state of the latest live release, or `None` before the first publish. A snapshot this version cannot read counts as empty,
/// so everything shows as new rather than hiding changes.
pub async fn live_release(db: &DatabaseConnection) -> Result<Option<LiveRelease>, StoreError> {
  let Some(row) = release::Entity::find()
    .filter(release::Column::Status.eq(ReleaseStatus::Live.as_str()))
    .order_by_desc(release::Column::Id)
    .one(db)
    .await?
  else {
    return Ok(None);
  };

  let revisions = release_page::Entity::find().filter(release_page::Column::ReleaseId.eq(row.id)).all(db).await?;

  Ok(Some(LiveRelease {
    pages: serde_json::from_str(&row.tree_snapshot).unwrap_or_default(),
    redirects: serde_json::from_str(&row.redirects_snapshot).unwrap_or_default(),
    revisions: revisions.into_iter().map(|r| (r.page_id.into(), r.revision_id.into())).collect(),
  }))
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

/// The theme settings, with defaults for anything not saved yet. They are one JSON document; settings saved by the first version of the
/// theme (separate name, logo and accent rows) are read when there is no document yet.
pub async fn load_settings(db: &DatabaseConnection) -> Result<Settings, StoreError> {
  if let Some(json) = get_setting(db, "site.settings").await? {
    return serde_json::from_str(&json).map_err(|e| StoreError::Invalid(format!("bad settings: {e}")));
  }

  let defaults = Settings::default();

  Ok(Settings {
    site_name: get_setting(db, "site.name").await?.unwrap_or(defaults.site_name.clone()),
    logo_asset_id: get_setting(db, "site.logo").await?.and_then(|id| id.parse::<AssetId>().ok()),
    accent: get_setting(db, "site.accent").await?.unwrap_or(defaults.accent.clone()),
    ..defaults
  })
}

pub async fn save_settings(db: &DatabaseConnection, settings: &Settings) -> Result<(), StoreError> {
  settings.validate().map_err(StoreError::Invalid)?;

  let settings = Settings {
    site_name: settings.site_name.trim().into(),
    accent: settings.accent.to_lowercase(),
    ..settings.clone()
  };
  let json = serde_json::to_string(&settings).map_err(|e| StoreError::Other(e.into()))?;

  set_setting(db, "site.settings", &json).await
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

    let saved = Settings {
      site_name: " Halyard ".into(),
      logo_asset_id: Some(AssetId::new()),
      accent: "#0B6E4F".into(),
      ..Settings::default()
    };
    save_settings(&db, &saved).await.unwrap();
    let loaded = load_settings(&db).await.unwrap();
    assert_eq!((loaded.site_name.as_str(), loaded.accent.as_str(), loaded.logo_asset_id), ("Halyard", "#0b6e4f", saved.logo_asset_id));
    assert!(matches!(save_settings(&db, &Settings { accent: "nope".into(), ..saved }).await, Err(StoreError::Invalid(_))));
  }
}
