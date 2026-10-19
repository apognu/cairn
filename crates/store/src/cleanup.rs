//! Finding uploads that nothing can show any more.

use std::collections::HashSet;

use cairn_core::{Asset, AssetId, ReleaseStatus};
use jiff::Timestamp;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use serde_json::Value;
use uuid::Uuid;

use crate::{
  StoreError,
  assets::into_asset,
  entity::{asset, draft, library_entry, release, release_page, revision, setting},
};

/// How many IDs go in one statement, well under the bound-parameter limits of both databases.
const CHUNK: usize = 500;

/// The assets uploaded before `before` that no draft, library entry, setting, release (other than a failed one) or revision saved
/// since `before` names. Older revisions that no release published do not hold an asset: restoring one of them may show a missing image.
/// Assets kept in the media library stay, and so does the image any staying asset was made from, through a whole chain of copies.
pub async fn unused_assets(db: &DatabaseConnection, before: Timestamp) -> Result<Vec<Asset>, StoreError> {
  let all = asset::Entity::find().all(db).await?;
  let old: Vec<&asset::Model> = all.iter().filter(|row| !row.kept && row.created_at.parse::<Timestamp>().is_ok_and(|at| at < before)).collect();
  if old.is_empty() {
    return Ok(Vec::new());
  }
  let mut unused: HashSet<String> = old.iter().map(|row| row.id.to_string()).collect();

  let mut forget = |text: &str| {
    if let Ok(value) = serde_json::from_str::<Value>(text) {
      forget_named(&value, &mut unused);
    }
  };

  for row in draft::Entity::find().all(db).await? {
    forget(&row.doc);
  }
  for row in library_entry::Entity::find().all(db).await? {
    if let Some(doc) = &row.doc {
      forget(doc);
    }
  }
  for row in setting::Entity::find().all(db).await? {
    forget(&row.value);
  }

  let releases = release::Entity::find().filter(release::Column::Status.ne(ReleaseStatus::Failed.as_str())).all(db).await?;
  for row in &releases {
    if let Some(settings) = &row.settings_snapshot {
      forget(settings);
    }
  }

  // The revisions a release published (it can be rolled back to) and the ones saved during the grace period.
  let mut held: HashSet<Uuid> = HashSet::new();
  let release_ids: Vec<Uuid> = releases.iter().map(|r| r.id).collect();
  for chunk in release_ids.chunks(CHUNK) {
    let rows = release_page::Entity::find().filter(release_page::Column::ReleaseId.is_in(chunk.to_vec())).all(db).await?;
    held.extend(rows.into_iter().map(|r| r.revision_id));
  }
  let saved: Vec<(Uuid, String)> = revision::Entity::find()
    .select_only()
    .columns([revision::Column::Id, revision::Column::CreatedAt])
    .into_tuple()
    .all(db)
    .await?;
  held.extend(saved.into_iter().filter(|(_, at)| !at.parse::<Timestamp>().is_ok_and(|at| at < before)).map(|(id, _)| id));

  let held: Vec<Uuid> = held.into_iter().collect();
  for chunk in held.chunks(CHUNK) {
    for row in revision::Entity::find().filter(revision::Column::Id.is_in(chunk.to_vec())).all(db).await? {
      forget(&row.doc);
    }
  }

  // A source stays when one of its copies stays; repeat until no more sources are freed, for chains of copies.
  loop {
    let sources: Vec<String> = all
      .iter()
      .filter(|row| !unused.contains(&row.id.to_string()))
      .filter_map(|row| row.derived_from.map(|id| id.to_string()))
      .filter(|id| unused.contains(id))
      .collect();
    if sources.is_empty() {
      break;
    }
    sources.iter().for_each(|id| {
      unused.remove(id);
    });
  }

  old.into_iter().filter(|row| unused.contains(&row.id.to_string())).cloned().map(into_asset).collect()
}

/// Deletes these asset rows; their files are the caller's to delete.
pub async fn delete_assets(db: &DatabaseConnection, ids: &[AssetId]) -> Result<(), StoreError> {
  for chunk in ids.chunks(CHUNK) {
    asset::Entity::delete_many().filter(asset::Column::Id.is_in(chunk.iter().map(|id| id.as_uuid()))).exec(db).await?;
  }

  Ok(())
}

/// Every hash an asset row has, to tell stored files that belong to no asset.
pub async fn asset_hashes(db: &DatabaseConnection) -> Result<HashSet<String>, StoreError> {
  let hashes: Vec<String> = asset::Entity::find().select_only().column(asset::Column::Sha256).into_tuple().all(db).await?;

  Ok(hashes.into_iter().collect())
}

/// Whether a string in `value` is `name`, the way `forget_named` finds it.
pub(crate) fn mentions(value: &Value, name: &str) -> bool {
  let mut wanted = HashSet::from([name.to_string()]);
  forget_named(value, &mut wanted);

  wanted.is_empty()
}

/// Removes from `unused` every ID a string in `value` names: an image node's `assetId`, a logo setting, or a component prop.
fn forget_named(value: &Value, unused: &mut HashSet<String>) {
  match value {
    Value::String(text) => {
      unused.remove(text.as_str());
    }
    Value::Array(items) => items.iter().for_each(|item| forget_named(item, unused)),
    Value::Object(fields) => fields.values().for_each(|field| forget_named(field, unused)),
    _ => {}
  }
}

#[cfg(test)]
mod tests {
  use cairn_core::{ContentStore, Document, PageId, Placement, Settings, UserId};
  use jiff::SignedDuration;
  use sea_orm::sea_query::Expr;

  use super::*;
  use crate::{NewAsset, SqlContentStore, add_release_pages, connect::test_db, create_release, get_or_create_asset, releases::ReleasePage, save_settings, set_release_status, tree::create_page};

  fn with_image(asset: AssetId) -> Document {
    serde_json::from_value(serde_json::json!({ "type": "doc", "content": [{ "type": "image", "attrs": { "assetId": asset, "alt": "" } }] })).unwrap()
  }

  async fn backdate_revision(db: &DatabaseConnection, id: cairn_core::RevisionId, days: i64) {
    let at = Timestamp::now() - SignedDuration::from_hours(24 * days);
    revision::Entity::update_many()
      .col_expr(revision::Column::CreatedAt, Expr::value(at.to_string()))
      .filter(revision::Column::Id.eq(id.as_uuid()))
      .exec(db)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn an_asset_goes_once_nothing_that_can_still_be_shown_names_it() {
    let db = test_db().await;
    let content = SqlContentStore::new(db.clone());
    let user = UserId::new();
    let names = [
      "in a draft",
      "nowhere",
      "uploaded recently",
      "in a recent version",
      "in an old version",
      "published",
      "logo",
      "in a failed release",
      "kept in the library",
      "source of a copy in a draft",
      "copy in a draft",
      "chain source",
      "chain middle",
      "chain copy kept",
      "source of a stray copy",
      "stray copy",
    ];
    // Copies point at what they were made from; only these are kept on purpose.
    let made_from = [
      ("copy in a draft", "source of a copy in a draft"),
      ("chain middle", "chain source"),
      ("chain copy kept", "chain middle"),
      ("stray copy", "source of a stray copy"),
    ];
    let kept = ["kept in the library", "chain copy kept"];

    let mut ids = Vec::new();
    for (n, name) in names.iter().enumerate() {
      let new = NewAsset {
        sha256: format!("{n:064}"),
        mime: "image/png".into(),
        width: Some(1),
        height: Some(1),
        bytes: 1,
        variants: vec![],
        placeholder: None,
        uploaded_by: user,
        name: None,
        title: None,
        derived_from: made_from.iter().find(|(copy, _)| copy == name).map(|(_, source)| ids[names.iter().position(|n| n == source).unwrap()]),
        kept: kept.contains(name),
      };
      ids.push(get_or_create_asset(&db, new).await.unwrap().0.id);
      if *name != "uploaded recently" {
        let at = Timestamp::now() - SignedDuration::from_hours(24 * 40);
        asset::Entity::update_many()
          .col_expr(asset::Column::CreatedAt, Expr::value(at.to_string()))
          .filter(asset::Column::Id.eq(ids[n].as_uuid()))
          .exec(&db)
          .await
          .unwrap();
      }
    }
    let id = |name: &str| ids[names.iter().position(|n| *n == name).unwrap()];

    // Saved with the image, then saved again without it: only the first version names it.
    let shown_then_removed = |name: &'static str| {
      let (db, content) = (db.clone(), content.clone());
      async move {
        let page = create_page(&db, user, None, name, Placement::Last).await.unwrap();
        let with = content.save_draft(page.id, with_image(id(name)), None, user).await.unwrap().base_revision_id.unwrap();
        content.save_draft(page.id, Document::paragraph("gone"), Some(with), user).await.unwrap();
        (page.id, with)
      }
    };

    let drafted = create_page(&db, user, None, "Drafted", Placement::Last).await.unwrap();
    content.save_draft(drafted.id, with_image(id("in a draft")), None, user).await.unwrap();
    let with_copy = create_page(&db, user, None, "With a copy", Placement::Last).await.unwrap();
    content.save_draft(with_copy.id, with_image(id("copy in a draft")), None, user).await.unwrap();

    shown_then_removed("in a recent version").await;
    let (_, old) = shown_then_removed("in an old version").await;
    backdate_revision(&db, old, 40).await;

    for (name, status) in [("published", ReleaseStatus::Superseded), ("in a failed release", ReleaseStatus::Failed)] {
      let (page_id, revision_id): (PageId, _) = shown_then_removed(name).await;
      backdate_revision(&db, revision_id, 40).await;
      let release = create_release(&db, user, "[]".into(), "[]".into(), &Settings::default()).await.unwrap();
      let page = ReleasePage {
        page_id,
        revision_id,
        output_hash: String::new(),
        content_hash: String::new(),
      };
      add_release_pages(&db, release.id, vec![page]).await.unwrap();
      set_release_status(&db, release.id, status).await.unwrap();
    }

    let settings = Settings {
      logo_asset_id: Some(id("logo")),
      ..Settings::default()
    };
    save_settings(&db, &settings).await.unwrap();

    let before = Timestamp::now() - SignedDuration::from_hours(24 * 30);
    let unused: HashSet<AssetId> = unused_assets(&db, before).await.unwrap().into_iter().map(|a| a.id).collect();

    for (name, gone) in [
      ("in a draft", false),
      ("nowhere", true),
      ("uploaded recently", false),
      ("in a recent version", false),
      ("in an old version", true),
      ("published", false),
      ("logo", false),
      ("in a failed release", true),
      ("kept in the library", false),
      ("source of a copy in a draft", false),
      ("copy in a draft", false),
      ("chain source", false),
      ("chain middle", false),
      ("chain copy kept", false),
      ("source of a stray copy", true),
      ("stray copy", true),
    ] {
      assert_eq!(unused.contains(&id(name)), gone, "{name}");
    }

    delete_assets(&db, &unused.into_iter().collect::<Vec<_>>()).await.unwrap();
    assert_eq!(asset_hashes(&db).await.unwrap().len(), 11);
  }
}
