//! Deleting pages for good once they have waited long enough in the trash.

use std::collections::{HashMap, HashSet};

use cairn_core::ReleaseStatus;
use jiff::Timestamp;
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait, sea_query::Expr};
use uuid::Uuid;

use crate::{
  StoreError,
  entity::{api_config, draft, entry_usage, library_entry, page, page_link, page_lock, page_review, redirect, release, release_api_spec, release_page, revision},
};

/// How many IDs go in one statement, well under the bound-parameter limits of both databases.
const CHUNK: usize = 500;

/// Deletes the pages that went to the trash before `before`, with their versions, draft, links and library usages; a term linking to one
/// loses its link. A page that the live release or a preview still shows stays until a publish takes it out, and so does a page with a
/// page under it that stays. Returns how many pages were deleted.
pub async fn purge_trash(db: &DatabaseConnection, before: Timestamp) -> Result<usize, StoreError> {
  let rows = page::Entity::find().all(db).await?;

  let kept_releases: Vec<Uuid> = release::Entity::find()
    .filter(release::Column::Status.is_in([ReleaseStatus::Live.as_str(), ReleaseStatus::Preview.as_str()]))
    .all(db)
    .await?
    .into_iter()
    .map(|r| r.id)
    .collect();
  let mut shown: HashSet<Uuid> = HashSet::new();
  for chunk in kept_releases.chunks(CHUNK) {
    let pages = release_page::Entity::find().filter(release_page::Column::ReleaseId.is_in(chunk.to_vec())).all(db).await?;
    let specs = release_api_spec::Entity::find().filter(release_api_spec::Column::ReleaseId.is_in(chunk.to_vec())).all(db).await?;
    shown.extend(pages.into_iter().map(|p| p.page_id).chain(specs.into_iter().map(|s| s.page_id)));
  }

  let expired = |row: &page::Model| row.deleted_at.as_deref().and_then(|at| at.parse::<Timestamp>().ok()).is_some_and(|at| at < before);
  let mut doomed: HashSet<Uuid> = rows.iter().filter(|r| expired(r) && !shown.contains(&r.id)).map(|r| r.id).collect();
  // The parent link of a page that stays must still point somewhere.
  loop {
    let held: Vec<Uuid> = rows.iter().filter(|r| !doomed.contains(&r.id)).filter_map(|r| r.parent_id).filter(|p| doomed.contains(p)).collect();
    if held.is_empty() {
      break;
    }
    for parent in held {
      doomed.remove(&parent);
    }
  }
  if doomed.is_empty() {
    return Ok(0);
  }

  // Deepest first, so no chunk deletes a parent before its children.
  let parents: HashMap<Uuid, Option<Uuid>> = rows.iter().map(|r| (r.id, r.parent_id)).collect();
  let depth = |id: Uuid| std::iter::successors(parents.get(&id).copied().flatten(), |p| parents.get(p).copied().flatten()).count();
  let mut ids: Vec<Uuid> = doomed.into_iter().collect();
  ids.sort_by_key(|id| std::cmp::Reverse(depth(*id)));

  let tx = db.begin().await?;
  // What refers to the pages goes first; drafts and release pages before the revisions they point at.
  for chunk in ids.chunks(CHUNK) {
    let chunk = chunk.to_vec();
    page_link::Entity::delete_many()
      .filter(
        Condition::any()
          .add(page_link::Column::SourcePageId.is_in(chunk.clone()))
          .add(page_link::Column::TargetPageId.is_in(chunk.clone())),
      )
      .exec(&tx)
      .await?;
    entry_usage::Entity::delete_many().filter(entry_usage::Column::PageId.is_in(chunk.clone())).exec(&tx).await?;
    redirect::Entity::delete_many().filter(redirect::Column::ToPageId.is_in(chunk.clone())).exec(&tx).await?;
    library_entry::Entity::update_many()
      .col_expr(library_entry::Column::LinkPageId, Expr::value(Option::<Uuid>::None))
      .filter(library_entry::Column::LinkPageId.is_in(chunk.clone()))
      .exec(&tx)
      .await?;
    draft::Entity::delete_many().filter(draft::Column::PageId.is_in(chunk.clone())).exec(&tx).await?;
    release_page::Entity::delete_many().filter(release_page::Column::PageId.is_in(chunk.clone())).exec(&tx).await?;
    release_api_spec::Entity::delete_many().filter(release_api_spec::Column::PageId.is_in(chunk.clone())).exec(&tx).await?;
    api_config::Entity::delete_many().filter(api_config::Column::PageId.is_in(chunk.clone())).exec(&tx).await?;
    page_lock::Entity::delete_many().filter(page_lock::Column::PageId.is_in(chunk.clone())).exec(&tx).await?;
    page_review::Entity::delete_many().filter(page_review::Column::PageId.is_in(chunk.clone())).exec(&tx).await?;
    revision::Entity::delete_many().filter(revision::Column::PageId.is_in(chunk)).exec(&tx).await?;
  }
  for chunk in ids.chunks(CHUNK) {
    page::Entity::delete_many().filter(page::Column::Id.is_in(chunk.to_vec())).exec(&tx).await?;
  }
  tx.commit().await?;

  Ok(ids.len())
}

#[cfg(test)]
mod tests {
  use cairn_core::{ContentStore, Document, PageId, Placement, Settings, UserId};
  use sea_orm::sea_query::Expr;

  use super::*;
  use crate::{
    SqlContentStore, add_release_pages, all_links,
    connect::test_db,
    create_release, list_trash,
    releases::ReleasePage,
    set_release_status,
    tree::{create_page, delete_page, list_pages},
  };

  async fn deleted_days_ago(db: &DatabaseConnection, id: PageId, days: i64) {
    delete_page(db, id, cairn_core::UserId::new()).await.unwrap();
    let at = Timestamp::now() - jiff::SignedDuration::from_hours(24 * days);
    page::Entity::update_many()
      .col_expr(page::Column::DeletedAt, Expr::value(at.to_string()))
      .filter(Condition::any().add(page::Column::Id.eq(id.as_uuid())).add(page::Column::ParentId.eq(id.as_uuid())))
      .exec(db)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn pages_trashed_more_than_90_days_ago_are_purged_unless_the_live_site_shows_them() {
    let db = test_db().await;
    let content = SqlContentStore::new(db.clone());
    let user = UserId::new();
    let page = |title: &'static str, parent: Option<PageId>| create_page(&db, user, parent, title, Placement::Last);

    let old = page("Old", None).await.unwrap();
    let old_child = page("Old child", Some(old.id)).await.unwrap();
    let recent = page("Recent", None).await.unwrap();
    let live = page("Still live", None).await.unwrap();
    let staying = page("Staying", None).await.unwrap();

    // Versions, a draft and a link pointing at the old page: all of it goes with it.
    content.save_draft(old.id, Document::paragraph("old"), None, user).await.unwrap();
    let link = serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
      { "type": "text", "text": "see", "marks": [{ "type": "link", "attrs": { "pageId": old.id } }] },
    ] }] });
    content.save_draft(staying.id, serde_json::from_value(link).unwrap(), None, user).await.unwrap();
    // The live site still has this one, deleted since.
    let revision = content.save_draft(live.id, Document::paragraph("live"), None, user).await.unwrap().base_revision_id.unwrap();
    let release = create_release(&db, user, "[]".into(), "[]".into(), &Settings::default()).await.unwrap();
    let shown = ReleasePage {
      page_id: live.id,
      revision_id: revision,
      output_hash: String::new(),
      content_hash: String::new(),
    };
    add_release_pages(&db, release.id, vec![shown]).await.unwrap();
    set_release_status(&db, release.id, ReleaseStatus::Live).await.unwrap();

    deleted_days_ago(&db, old.id, 91).await;
    deleted_days_ago(&db, recent.id, 10).await;
    deleted_days_ago(&db, live.id, 120).await;

    let purged = purge_trash(&db, Timestamp::now() - jiff::SignedDuration::from_hours(24 * 90)).await.unwrap();
    assert_eq!(purged, 2);

    let all: Vec<Uuid> = page::Entity::find().all(&db).await.unwrap().into_iter().map(|p| p.id).collect();
    assert!(!all.contains(&old.id.as_uuid()) && !all.contains(&old_child.id.as_uuid()));
    let trash: Vec<String> = list_trash(&db).await.unwrap().into_iter().map(|t| t.page.title).collect();
    assert_eq!(trash, ["Recent", "Still live"]);
    assert!(all_links(&db).await.unwrap().is_empty());
    assert_eq!(list_pages(&db).await.unwrap().len(), 1);
  }
}
