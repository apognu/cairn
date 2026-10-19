use cairn_core::{Asset, AssetId, AssetVariant, PageId, UserId};
use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, sea_query::LikeExpr};
use serde_json::Value;

use crate::{
  StoreError,
  cleanup::mentions,
  entity::{asset, draft, library_entry, page, setting},
};

/// Longest title or file name kept for an asset.
const MAX_LABEL: usize = 200;

/// What is known about a file before it has an ID.
pub struct NewAsset {
  pub sha256: String,
  pub mime: String,
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub bytes: u64,
  pub variants: Vec<AssetVariant>,
  pub placeholder: Option<String>,
  pub uploaded_by: UserId,
  pub name: Option<String>,
  pub title: Option<String>,
  pub derived_from: Option<AssetId>,
  pub kept: bool,
}

/// What still shows an asset, so it cannot be deleted from the media library.
#[derive(Debug, Default, PartialEq)]
pub struct AssetUsage {
  /// Pages whose draft shows it, by title.
  pub pages: Vec<(PageId, String)>,
  /// Library entries (snippets) that show it, by key.
  pub entries: Vec<String>,
  /// The site settings name it (the logo).
  pub settings: bool,
  /// Kept copies made from it.
  pub copies: Vec<AssetId>,
}

impl AssetUsage {
  pub fn is_empty(&self) -> bool {
    self.pages.is_empty() && self.entries.is_empty() && !self.settings && self.copies.is_empty()
  }
}

pub async fn asset_by_id(db: &DatabaseConnection, id: AssetId) -> Result<Asset, StoreError> {
  let row = asset::Entity::find_by_id(id.as_uuid()).one(db).await?;

  into_asset(row.ok_or(StoreError::NotFound)?)
}

/// The asset with this content hash, if those bytes were uploaded before.
pub async fn asset_by_hash(db: &DatabaseConnection, sha256: &str) -> Result<Option<Asset>, StoreError> {
  by_hash(db, sha256).await
}

/// The assets with these IDs; unknown IDs are left out.
pub async fn assets_by_ids(db: &DatabaseConnection, ids: &[AssetId]) -> Result<Vec<Asset>, StoreError> {
  let rows = asset::Entity::find().filter(asset::Column::Id.is_in(ids.iter().map(|id| id.as_uuid()))).all(db).await?;

  rows.into_iter().map(into_asset).collect()
}

/// Assets newest first, `limit` at a time, starting after `before`. `search` matches part of the title or of the file name, ignoring case.
pub async fn list_assets(db: &DatabaseConnection, search: Option<&str>, before: Option<AssetId>, limit: u64) -> Result<Vec<Asset>, StoreError> {
  // IDs are UUIDv7, so ordering by ID is ordering by upload time, and the last ID of a page is where the next one starts.
  let mut query = asset::Entity::find().order_by_desc(asset::Column::Id).limit(limit);

  if let Some(text) = search.map(str::trim).filter(|text| !text.is_empty()) {
    query = query.filter(asset::Column::Search.like(LikeExpr::new(format!("%{}%", escape_like(&text.to_lowercase()))).escape('\\')));
  }
  if let Some(before) = before {
    query = query.filter(asset::Column::Id.lt(before.as_uuid()));
  }

  query.all(db).await?.into_iter().map(into_asset).collect()
}

/// Names an asset; an empty title removes the name.
pub async fn set_asset_title(db: &DatabaseConnection, id: AssetId, title: &str) -> Result<Asset, StoreError> {
  let row = asset::Entity::find_by_id(id.as_uuid()).one(db).await?.ok_or(StoreError::NotFound)?;
  let title = label(Some(title));
  let search = search_text(title.as_deref(), row.name.as_deref());

  let mut active: asset::ActiveModel = row.into();
  active.title = Set(title);
  active.search = Set(search);

  into_asset(active.update(db).await?)
}

/// Everything that still shows the asset: page drafts, library entries, the settings, and kept copies made from it.
pub async fn asset_usage(db: &DatabaseConnection, id: AssetId) -> Result<AssetUsage, StoreError> {
  let name = id.to_string();
  let shows = |text: &str| serde_json::from_str::<Value>(text).is_ok_and(|value| mentions(&value, &name));
  let mut usage = AssetUsage::default();

  let live: Vec<(uuid::Uuid, String)> = page::Entity::find()
    .select_only()
    .columns([page::Column::Id, page::Column::Title])
    .filter(page::Column::DeletedAt.is_null())
    .into_tuple()
    .all(db)
    .await?;
  let drafts = draft::Entity::find().all(db).await?;
  for (page_id, title) in live {
    if drafts.iter().any(|d| d.page_id == page_id && shows(&d.doc)) {
      usage.pages.push((page_id.into(), title));
    }
  }
  usage.pages.sort_by(|a, b| a.1.cmp(&b.1));

  for row in library_entry::Entity::find().all(db).await? {
    if row.doc.as_deref().is_some_and(shows) {
      usage.entries.push(row.key);
    }
  }
  usage.settings = setting::Entity::find().all(db).await?.iter().any(|row| shows(&row.value));
  usage.copies = asset::Entity::find()
    .filter(asset::Column::DerivedFrom.eq(id.as_uuid()))
    .filter(asset::Column::Kept.eq(true))
    .all(db)
    .await?
    .into_iter()
    .map(|row| row.id.into())
    .collect();

  Ok(usage)
}

/// Records an uploaded file, or returns the existing asset when the same bytes were uploaded before. The boolean says which happened.
/// Uploading known bytes into the library keeps the existing asset.
pub async fn get_or_create_asset(db: &DatabaseConnection, new: NewAsset) -> Result<(Asset, bool), StoreError> {
  if let Some(existing) = by_hash(db, &new.sha256).await? {
    return Ok((keep_if(db, existing, new.kept).await?, false));
  }

  let name = label(new.name.as_deref());
  let title = label(new.title.as_deref());

  let inserted = asset::ActiveModel {
    id: Set(AssetId::new().as_uuid()),
    sha256: Set(new.sha256.clone()),
    mime: Set(new.mime),
    width: Set(new.width.map(i64::from)),
    height: Set(new.height.map(i64::from)),
    bytes: Set(new.bytes as i64),
    variants: Set(serde_json::to_string(&new.variants).map_err(|e| StoreError::Other(e.into()))?),
    placeholder: Set(new.placeholder),
    uploaded_by: Set(new.uploaded_by.as_uuid()),
    created_at: Set(Timestamp::now().to_string()),
    search: Set(search_text(title.as_deref(), name.as_deref())),
    title: Set(title),
    name: Set(name),
    derived_from: Set(new.derived_from.map(|id| id.as_uuid())),
    kept: Set(new.kept),
  }
  .insert(db)
  .await;

  match inserted {
    Ok(row) => Ok((into_asset(row)?, true)),
    // Two uploads of the same file raced and the other won the unique hash.
    Err(error) => match by_hash(db, &new.sha256).await? {
      Some(existing) => Ok((keep_if(db, existing, new.kept).await?, false)),
      None => Err(error.into()),
    },
  }
}

async fn keep_if(db: &DatabaseConnection, existing: Asset, kept: bool) -> Result<Asset, StoreError> {
  if !kept || existing.kept {
    return Ok(existing);
  }

  set_kept(db, existing.id, true).await?;
  Ok(Asset { kept: true, ..existing })
}

/// Puts an asset in the media library, or takes it out: cleanup then removes it once nothing shows it, after the grace period.
pub async fn set_kept(db: &DatabaseConnection, id: AssetId, kept: bool) -> Result<(), StoreError> {
  asset::Entity::update_many()
    .col_expr(asset::Column::Kept, sea_orm::sea_query::Expr::value(kept))
    .filter(asset::Column::Id.eq(id.as_uuid()))
    .exec(db)
    .await?;

  Ok(())
}

/// Trims a title or file name and caps its length; blank is none.
fn label(text: Option<&str>) -> Option<String> {
  let text = text?.trim();

  (!text.is_empty()).then(|| text.chars().take(MAX_LABEL).collect())
}

fn search_text(title: Option<&str>, name: Option<&str>) -> String {
  [title, name].into_iter().flatten().collect::<Vec<_>>().join("\n").to_lowercase()
}

/// Makes `%`, `_` and the escape itself literal in a LIKE pattern.
fn escape_like(text: &str) -> String {
  text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

async fn by_hash(db: &DatabaseConnection, sha256: &str) -> Result<Option<Asset>, StoreError> {
  let row = asset::Entity::find().filter(asset::Column::Sha256.eq(sha256)).one(db).await?;

  row.map(into_asset).transpose()
}

pub(crate) fn into_asset(row: asset::Model) -> Result<Asset, StoreError> {
  Ok(Asset {
    id: row.id.into(),
    sha256: row.sha256,
    mime: row.mime,
    width: row.width.map(|w| w as u32),
    height: row.height.map(|h| h as u32),
    bytes: row.bytes as u64,
    variants: serde_json::from_str(&row.variants).map_err(|e| StoreError::Invalid(format!("bad variants: {e}")))?,
    placeholder: row.placeholder,
    uploaded_by: row.uploaded_by.into(),
    created_at: row.created_at.parse().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")))?,
    title: row.title,
    name: row.name,
    derived_from: row.derived_from.map(Into::into),
    kept: row.kept,
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
      variants: vec![AssetVariant { width: 1, height: 1, bytes: 5 }],
      placeholder: Some("#112233".into()),
      uploaded_by: UserId::new(),
      name: None,
      title: None,
      derived_from: None,
      kept: false,
    };

    let (first, created) = get_or_create_asset(&db, new()).await.unwrap();
    let (second, created_again) = get_or_create_asset(&db, new()).await.unwrap();

    assert!(created && !created_again);
    assert_eq!((first.variants.len(), first.placeholder.as_deref()), (1, Some("#112233")));
    assert_eq!(first.id, second.id);
    assert_eq!(asset_by_id(&db, first.id).await.unwrap().mime, "image/png");
  }

  #[tokio::test]
  async fn the_library_is_searched_by_title_and_file_name_and_paged_newest_first() {
    let db = test_db().await;
    let user = UserId::new();
    let mut ids = Vec::new();
    for (n, (name, title)) in [("Login-Screen.PNG", None), ("dashboard.png", Some("Main Dashboard")), ("50%_off.jpg", None)].into_iter().enumerate() {
      let new = NewAsset {
        sha256: format!("{n:064}"),
        mime: "image/png".into(),
        width: None,
        height: None,
        bytes: 1,
        variants: vec![],
        placeholder: None,
        uploaded_by: user,
        name: Some(name.into()),
        title: title.map(Into::into),
        derived_from: None,
        kept: true,
      };
      ids.push(get_or_create_asset(&db, new).await.unwrap().0.id);
    }
    let found = |search: &'static str| {
      let db = db.clone();
      async move { list_assets(&db, Some(search), None, 10).await.unwrap().into_iter().map(|a| a.id).collect::<Vec<_>>() }
    };

    for (search, expected) in [
      ("login", vec![ids[0]]),
      ("main dash", vec![ids[1]]),
      ("PNG", vec![ids[1], ids[0]]),
      ("%", vec![ids[2]]),
      ("_", vec![ids[2]]),
      ("", ids.iter().rev().copied().collect()),
    ] {
      assert_eq!(found(search).await, expected, "{search:?}");
    }

    let renamed = set_asset_title(&db, ids[0], "  Sign in  ").await.unwrap();
    assert_eq!(renamed.title.as_deref(), Some("Sign in"));
    assert_eq!(found("sign in").await, vec![ids[0]]);
    assert_eq!(found("login").await, vec![ids[0]], "the file name is still searched");
    assert_eq!(set_asset_title(&db, ids[0], " ").await.unwrap().title, None);

    let first = list_assets(&db, None, None, 2).await.unwrap();
    let rest = list_assets(&db, None, Some(first[1].id), 2).await.unwrap();
    assert_eq!([first, rest].concat().iter().map(|a| a.id).collect::<Vec<_>>(), ids.iter().rev().copied().collect::<Vec<_>>());
  }
}
