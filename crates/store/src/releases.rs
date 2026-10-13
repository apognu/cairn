use std::collections::HashMap;

use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait, sea_query::OnConflict};
use yourdocs_core::{ApiConfig, AssetId, Page, PageId, Release, ReleaseId, ReleaseStatus, RevisionId, Settings, UserId};

use crate::{
  StoreError,
  entity::{release, release_api_spec, release_page, setting},
};

/// Starts a release. `tree_snapshot` and `redirects_snapshot` are JSON; they and the settings are frozen with the release so it can be
/// rebuilt or rolled back later, and so "what changed" has something to compare with.
pub async fn create_release(db: &DatabaseConnection, user: UserId, tree_snapshot: String, redirects_snapshot: String, settings: &Settings) -> Result<Release, StoreError> {
  let base = latest_live_release(db).await?;
  let settings = serde_json::to_string(settings).map_err(|e| StoreError::Other(e.into()))?;

  let row = release::ActiveModel {
    id: Set(ReleaseId::new().as_uuid()),
    base_release_id: Set(base.map(|r| r.id.as_uuid())),
    created_by: Set(user.as_uuid()),
    created_at: Set(Timestamp::now().to_string()),
    tree_snapshot: Set(tree_snapshot),
    redirects_snapshot: Set(redirects_snapshot),
    status: Set(ReleaseStatus::Building.as_str().into()),
    target_version_id: Set(None),
    settings_snapshot: Set(Some(settings)),
    preview_url: Set(None),
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

/// Records what the target answered for a release that was handed over: its version there and, for a preview, where to look at it.
pub async fn set_release_deployment(db: &DatabaseConnection, id: ReleaseId, status: ReleaseStatus, version: Option<String>, preview_url: Option<String>) -> Result<(), StoreError> {
  release::Entity::update_many()
    .col_expr(release::Column::Status, status.as_str().into())
    .col_expr(release::Column::TargetVersionId, version.into())
    .col_expr(release::Column::PreviewUrl, preview_url.into())
    .filter(release::Column::Id.eq(id.as_uuid()))
    .exec(db)
    .await?;

  Ok(())
}

/// Makes a release the live one and the previous live release a superseded one, together.
pub async fn make_release_live(db: &DatabaseConnection, id: ReleaseId) -> Result<(), StoreError> {
  let transaction = db.begin().await?;

  release::Entity::update_many()
    .col_expr(release::Column::Status, ReleaseStatus::Superseded.as_str().into())
    .filter(release::Column::Status.eq(ReleaseStatus::Live.as_str()))
    .exec(&transaction)
    .await?;
  release::Entity::update_many()
    .col_expr(release::Column::Status, ReleaseStatus::Live.as_str().into())
    .filter(release::Column::Id.eq(id.as_uuid()))
    .exec(&transaction)
    .await?;
  transaction.commit().await?;

  Ok(())
}

/// A release and its version at the target.
pub async fn get_release(db: &DatabaseConnection, id: ReleaseId) -> Result<(Release, Option<String>), StoreError> {
  let row = release::Entity::find_by_id(id.as_uuid()).one(db).await?.ok_or(StoreError::NotFound)?;
  let version = row.target_version_id.clone();

  Ok((into_release(db, row).await?, version))
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

/// What the live release published: its tree and redirect rules as frozen at the time, the revision of each page, and the settings it was
/// built with. Releases made before settings were recorded have `None` and no API settings.
pub struct LiveRelease {
  pub id: ReleaseId,
  pub pages: Vec<Page>,
  pub redirects: Vec<(String, String)>,
  pub revisions: HashMap<PageId, RevisionId>,
  pub settings: Option<Settings>,
  pub api_configs: HashMap<PageId, ApiConfig>,
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
  let specs = release_api_spec::Entity::find().filter(release_api_spec::Column::ReleaseId.eq(row.id)).all(db).await?;
  let api_configs = specs
    .into_iter()
    .filter_map(|spec| Some((spec.page_id.into(), serde_json::from_str(spec.config.as_deref()?).ok()?)))
    .collect();

  Ok(Some(LiveRelease {
    id: row.id.into(),
    pages: serde_json::from_str(&row.tree_snapshot).unwrap_or_default(),
    redirects: serde_json::from_str(&row.redirects_snapshot).unwrap_or_default(),
    revisions: revisions.into_iter().map(|r| (r.page_id.into(), r.revision_id.into())).collect(),
    settings: row.settings_snapshot.as_deref().and_then(|json| serde_json::from_str(json).ok()),
    api_configs,
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
    preview_url: row.preview_url,
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

    let release = create_release(&db, UserId::new(), "[]".into(), "[]".into(), &Settings::default()).await.unwrap();
    assert_eq!(release.status, ReleaseStatus::Building);
    assert!(latest_live_release(&db).await.unwrap().is_none());

    set_release_status(&db, release.id, ReleaseStatus::Live).await.unwrap();
    let live = latest_live_release(&db).await.unwrap().unwrap();
    assert_eq!(live.id, release.id);
    assert_eq!(
      create_release(&db, UserId::new(), "[]".into(), "[]".into(), &Settings::default()).await.unwrap().base_release_id,
      Some(release.id)
    );

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
