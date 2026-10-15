use std::collections::{HashMap, HashSet};

use cairn_core::{ApiConfig, AssetId, Page, PageId, Release, ReleaseId, ReleaseStatus, RevisionId, Settings, UserId};
use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait, sea_query::OnConflict};

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
    render_keys: Set(None),
    file_count: Set(None),
    bytes: Set(None),
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

/// One page of a release: the revision it was built from, the hash of its HTML, and the hash of its content (see `LiveRelease::content`).
pub struct ReleasePage {
  pub page_id: PageId,
  pub revision_id: RevisionId,
  pub output_hash: String,
  pub content_hash: String,
}

pub async fn add_release_pages(db: &DatabaseConnection, id: ReleaseId, pages: Vec<ReleasePage>) -> Result<(), StoreError> {
  if pages.is_empty() {
    return Ok(());
  }

  let rows = pages.into_iter().map(|page| release_page::ActiveModel {
    release_id: Set(id.as_uuid()),
    page_id: Set(page.page_id.as_uuid()),
    revision_id: Set(page.revision_id.as_uuid()),
    output_hash: Set(Some(page.output_hash)),
    content_hash: Set(Some(page.content_hash)),
  });
  release_page::Entity::insert_many(rows).exec(db).await?;

  Ok(())
}

/// Notes which render cache entries a release's pages came from.
pub async fn set_release_render_keys(db: &DatabaseConnection, id: ReleaseId, keys: &[String]) -> Result<(), StoreError> {
  release::Entity::update_many()
    .col_expr(release::Column::RenderKeys, serde_json::to_string(keys).map_err(|e| StoreError::Other(e.into()))?.into())
    .filter(release::Column::Id.eq(id.as_uuid()))
    .exec(db)
    .await?;

  Ok(())
}

/// Records how many files a release sent and their total size.
pub async fn set_release_stats(db: &DatabaseConnection, id: ReleaseId, files: u64, bytes: u64) -> Result<(), StoreError> {
  release::Entity::update_many()
    .col_expr(release::Column::FileCount, i64::try_from(files).unwrap_or(i64::MAX).into())
    .col_expr(release::Column::Bytes, i64::try_from(bytes).unwrap_or(i64::MAX).into())
    .filter(release::Column::Id.eq(id.as_uuid()))
    .exec(db)
    .await?;

  Ok(())
}

/// The render cache entries to keep: those of the live release and of the previews made after it, which can still be promoted. `None`
/// when one of them was made before entries were recorded, and nothing can be known.
pub async fn kept_render_keys(db: &DatabaseConnection) -> Result<Option<HashSet<String>>, StoreError> {
  let Some(live) = release::Entity::find()
    .filter(release::Column::Status.eq(ReleaseStatus::Live.as_str()))
    .order_by_desc(release::Column::Id)
    .one(db)
    .await?
  else {
    return Ok(Some(HashSet::new()));
  };
  let previews = release::Entity::find()
    .filter(release::Column::Status.eq(ReleaseStatus::Preview.as_str()))
    .filter(release::Column::Id.gt(live.id))
    .all(db)
    .await?;

  let mut kept = HashSet::new();
  for row in std::iter::once(live).chain(previews) {
    let Some(keys) = row.render_keys.as_deref().and_then(|json| serde_json::from_str::<Vec<String>>(json).ok()) else {
      return Ok(None);
    };
    kept.extend(keys);
  }

  Ok(Some(kept))
}

pub async fn latest_live_release(db: &impl ConnectionTrait) -> Result<Option<Release>, StoreError> {
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
  /// What each page's content was made of (expanded document, linked pages, terms), hashed; pages of releases made before it was recorded
  /// have none.
  pub content: HashMap<PageId, String>,
  pub settings: Option<Settings>,
  pub api_configs: HashMap<PageId, ApiConfig>,
}

/// The state of the latest live release, or `None` before the first publish. A snapshot this version cannot read counts as empty,
/// so everything shows as new rather than hiding changes.
pub async fn live_release(db: &impl ConnectionTrait) -> Result<Option<LiveRelease>, StoreError> {
  let Some(row) = release::Entity::find()
    .filter(release::Column::Status.eq(ReleaseStatus::Live.as_str()))
    .order_by_desc(release::Column::Id)
    .one(db)
    .await?
  else {
    return Ok(None);
  };

  Ok(Some(release_state(db, row).await?))
}

/// What any release published, live or not: a preview is compared with the staging it was made from this way.
pub async fn release_state_of(db: &impl ConnectionTrait, id: ReleaseId) -> Result<LiveRelease, StoreError> {
  let row = release::Entity::find_by_id(id.as_uuid()).one(db).await?.ok_or(StoreError::NotFound)?;

  release_state(db, row).await
}

async fn release_state(db: &impl ConnectionTrait, row: release::Model) -> Result<LiveRelease, StoreError> {
  let revisions = release_page::Entity::find().filter(release_page::Column::ReleaseId.eq(row.id)).all(db).await?;
  let specs = release_api_spec::Entity::find().filter(release_api_spec::Column::ReleaseId.eq(row.id)).all(db).await?;
  let api_configs = specs
    .into_iter()
    .filter_map(|spec| Some((spec.page_id.into(), serde_json::from_str(spec.config.as_deref()?).ok()?)))
    .collect();

  Ok(LiveRelease {
    id: row.id.into(),
    pages: serde_json::from_str(&row.tree_snapshot).unwrap_or_default(),
    redirects: serde_json::from_str(&row.redirects_snapshot).unwrap_or_default(),
    content: revisions.iter().filter_map(|r| Some((r.page_id.into(), r.content_hash.clone()?))).collect(),
    revisions: revisions.into_iter().map(|r| (r.page_id.into(), r.revision_id.into())).collect(),
    settings: row.settings_snapshot.as_deref().and_then(|json| serde_json::from_str(json).ok()),
    api_configs,
  })
}

async fn into_release(db: &impl ConnectionTrait, row: release::Model) -> Result<Release, StoreError> {
  let page_count = release_page::Entity::find().filter(release_page::Column::ReleaseId.eq(row.id)).count(db).await?;

  Ok(Release {
    id: row.id.into(),
    base_release_id: row.base_release_id.map(Into::into),
    created_by: row.created_by.into(),
    created_at: row.created_at.parse().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")))?,
    status: ReleaseStatus::parse(&row.status).ok_or_else(|| StoreError::Invalid(format!("unknown release status {}", row.status)))?,
    page_count: page_count as u32,
    preview_url: row.preview_url,
    file_count: row.file_count.and_then(|n| u64::try_from(n).ok()),
    bytes: row.bytes.and_then(|n| u64::try_from(n).ok()),
  })
}

pub async fn get_setting(db: &impl ConnectionTrait, key: &str) -> Result<Option<String>, StoreError> {
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
pub async fn load_settings(db: &impl ConnectionTrait) -> Result<Settings, StoreError> {
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
