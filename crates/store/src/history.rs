//! A page's saved versions, and the trash of deleted pages.

use std::collections::HashMap;

use cairn_core::{Page, PageId, Placement, ReleaseId, RevisionId, UserId, page_urls};
use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};

use crate::{
  StoreError,
  entity::{page, release, release_page, revision},
  tree::{into_page, last_header_position, list_pages, position_for},
  urls::check_page_urls,
};

/// One saved version of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
  pub id: RevisionId,
  pub author: UserId,
  pub created_at: Timestamp,
  pub title: String,
  /// The live release this version went out in, if it did (the most recent one).
  pub released_in: Option<(ReleaseId, Timestamp)>,
}

fn parse(text: &str) -> Result<Timestamp, StoreError> {
  text.parse().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")))
}

/// Every saved version of the page, newest first.
pub async fn page_history(db: &DatabaseConnection, page: PageId) -> Result<Vec<HistoryEntry>, StoreError> {
  let rows = revision::Entity::find()
    .filter(revision::Column::PageId.eq(page.as_uuid()))
    .order_by_desc(revision::Column::Id)
    .all(db)
    .await?;

  // Which releases each version went out in, keeping the newest. By page, not by a list of revision IDs, which a long-edited page would
  // push past the database's limit on bound parameters.
  let released = release_page::Entity::find().filter(release_page::Column::PageId.eq(page.as_uuid())).all(db).await?;
  let releases: HashMap<_, _> = release::Entity::find().all(db).await?.into_iter().filter(|r| r.status == "live").map(|r| (r.id, r)).collect();
  let mut released_in: HashMap<uuid::Uuid, (ReleaseId, Timestamp)> = HashMap::new();
  for rp in released {
    if let Some(r) = releases.get(&rp.release_id) {
      let at = parse(&r.created_at)?;
      if released_in.get(&rp.revision_id).is_none_or(|(_, seen)| *seen < at) {
        released_in.insert(rp.revision_id, (r.id.into(), at));
      }
    }
  }

  rows
    .into_iter()
    .map(|r| {
      Ok(HistoryEntry {
        id: r.id.into(),
        author: r.author_id.into(),
        created_at: parse(&r.created_at)?,
        title: r.title,
        released_in: released_in.get(&r.id).copied(),
      })
    })
    .collect()
}

/// Gives a page back the title one of its versions had. Its URL stays: a restore brings back content, not a move.
pub async fn restore_title(db: &DatabaseConnection, id: PageId, title: &str) -> Result<Page, StoreError> {
  if !list_pages(db).await?.iter().any(|p| p.id == id) {
    return Err(StoreError::NotFound);
  }
  if title.trim().is_empty() {
    return Err(StoreError::Invalid("a title cannot be empty".into()));
  }

  let row = page::ActiveModel {
    id: Set(id.as_uuid()),
    title: Set(title.to_string()),
    updated_at: Set(Timestamp::now().to_string()),
    ..Default::default()
  }
  .update(db)
  .await?;

  into_page(row)
}

/// A page waiting in the trash.
#[derive(Debug, Clone, PartialEq)]
pub struct Trashed {
  pub page: Page,
  pub deleted_at: Timestamp,
  /// The URL it had before it was deleted.
  pub old_url: String,
  /// How many pages went with it.
  pub subpages: usize,
}

async fn all_rows(db: &DatabaseConnection) -> Result<Vec<page::Model>, StoreError> {
  Ok(page::Entity::find().all(db).await?)
}

/// The pages deleted directly (not as part of a deleted parent's subtree), most recently deleted first.
pub async fn list_trash(db: &DatabaseConnection) -> Result<Vec<Trashed>, StoreError> {
  let rows = all_rows(db).await?;
  let pages: Vec<Page> = rows.iter().cloned().map(into_page).collect::<Result<_, _>>()?;
  let urls = page_urls(&pages);
  let deleted_with: HashMap<_, _> = rows.iter().map(|r| (r.id, r.deleted_at.clone())).collect();

  let mut found: Vec<Trashed> = Vec::new();
  for row in rows.iter().filter(|r| r.deleted_at.is_some()) {
    // Deleted together with its parent: it comes back with the parent, not on its own.
    let with_parent = row.parent_id.and_then(|p| deleted_with.get(&p)).is_some_and(|d| *d == row.deleted_at);
    if with_parent {
      continue;
    }

    let page = into_page(row.clone())?;
    let subpages = rows.iter().filter(|r| r.deleted_at == row.deleted_at && r.id != row.id && descends_from(&rows, r, row.id)).count();
    found.push(Trashed {
      old_url: urls.get(&page.id).cloned().unwrap_or_default(),
      deleted_at: parse(row.deleted_at.as_deref().unwrap_or_default())?,
      page,
      subpages,
    });
  }
  found.sort_by_key(|t| std::cmp::Reverse(t.deleted_at));

  Ok(found)
}

fn descends_from(rows: &[page::Model], row: &page::Model, ancestor: uuid::Uuid) -> bool {
  let mut current = row.parent_id;

  for _ in 0..64 {
    let Some(id) = current else { return false };
    if id == ancestor {
      return true;
    }
    current = rows.iter().find(|r| r.id == id).and_then(|r| r.parent_id);
  }

  false
}

/// Brings a deleted page back with the pages that were deleted with it, at its old place and with its old URL. When the old parent is
/// gone it returns to the top level, and when a sibling has taken its URL segment it gets a numbered one; the boolean says the old
/// place and URL were kept.
pub async fn restore_page(db: &DatabaseConnection, id: PageId) -> Result<(Page, bool), StoreError> {
  let rows = all_rows(db).await?;
  let row = rows.iter().find(|r| r.id == id.as_uuid() && r.deleted_at.is_some()).ok_or(StoreError::NotFound)?;
  let live = list_pages(db).await?;

  let parent = row.parent_id.filter(|p| live.iter().any(|l| l.id.as_uuid() == *p));
  let subtree: Vec<_> = rows.iter().filter(|r| r.deleted_at == row.deleted_at && (r.id == row.id || descends_from(&rows, r, row.id))).collect();

  // The slug must be free among the new siblings and must not give the page, or anything below it, a URL a live page has now (a custom
  // URL, say, set after the page was deleted). A clash is dodged with a suffix; one that a suffix cannot fix (deeper down) refuses the restore.
  let mut slug = row.slug.clone();
  let mut n = 1;
  loop {
    let mut after = live.clone();
    for r in &subtree {
      let mut page = into_page((*r).clone())?;
      if r.id == row.id {
        page.parent_id = parent.map(PageId::from);
        page.slug = slug.clone();
      }
      after.push(page);
    }

    let taken = live.iter().any(|l| l.parent_id.map(|p| p.as_uuid()) == parent && l.slug == slug);
    match check_page_urls(&live, &after) {
      Ok(()) if !taken => break,
      Err(e) if n >= 20 => return Err(e),
      _ => {}
    }

    n += 1;
    slug = format!("{}-{n}", row.slug);
  }
  let kept = parent == row.parent_id && slug == row.slug;

  // Positions are computed from the live pages only, so a page added while this one was in the trash can hold its old position (every
  // first child gets the same one). Two equal positions leave the order to chance and refuse any drop between them: the page then goes
  // just before the sibling that took its place.
  let parent_id = parent.map(PageId::from);
  let position = match live.iter().find(|l| l.parent_id == parent_id && l.position == row.position) {
    Some(taken) => position_for(&live, parent_id, None, Placement::Before(taken.id))?,
    None => row.position.clone(),
  };
  let header_taken = row.header_position.is_some() && live.iter().any(|l| l.in_header && l.header_position == row.header_position);
  let header_position = if header_taken { Some(last_header_position(&live)?) } else { row.header_position.clone() };

  // All of the subtree comes back, or none of it.
  let tx = db.begin().await?;
  for r in subtree {
    let mut active = page::ActiveModel {
      id: Set(r.id),
      deleted_at: Set(None),
      updated_at: Set(Timestamp::now().to_string()),
      ..Default::default()
    };
    if r.id == row.id {
      active.parent_id = Set(parent);
      active.slug = Set(slug.clone());
      active.position = Set(position.clone());
      active.header_position = Set(header_position.clone());
    }
    active.update(&tx).await?;
  }
  tx.commit().await?;

  let restored = list_pages(db).await?.into_iter().find(|p| p.id == id).ok_or(StoreError::NotFound)?;

  Ok((restored, kept))
}

#[cfg(test)]
mod tests {
  use cairn_core::{Placement, page_urls};

  use super::*;
  use crate::{
    connect::test_db,
    tree::{create_page, delete_page},
  };

  #[tokio::test]
  async fn a_restored_page_does_not_take_a_custom_url_set_since() {
    let db = test_db().await;
    let user = UserId::new();

    let hooks = create_page(&db, user, None, "Hooks", Placement::Last).await.unwrap();
    delete_page(&db, hooks.id, cairn_core::UserId::new()).await.unwrap();
    let other = create_page(&db, user, None, "Webhooks", Placement::Last).await.unwrap();
    crate::set_custom_url(&db, other.id, Some("/hooks/"), cairn_core::OldUrls::Redirect).await.unwrap();

    let (restored, kept) = restore_page(&db, hooks.id).await.unwrap();
    assert_eq!(restored.slug, "hooks-2");
    assert!(!kept);
  }

  #[tokio::test]
  async fn a_restored_page_gets_its_old_place_and_url_back() {
    let db = test_db().await;
    let user = UserId::new();
    let guide = create_page(&db, user, None, "Guide", Placement::Last).await.unwrap();
    let install = create_page(&db, user, Some(guide.id), "Install", Placement::Last).await.unwrap();
    let linux = create_page(&db, user, Some(install.id), "Linux", Placement::Last).await.unwrap();
    let url = |pages: &[Page], id: PageId| page_urls(pages)[&id].clone();
    // Two siblings with one position have no order, and nothing can be dropped between them.
    let distinct_positions = |pages: Vec<Page>| {
      let mut seen = std::collections::HashSet::new();
      pages.iter().all(|p| seen.insert((p.parent_id, p.position.clone())))
    };
    let before = url(&list_pages(&db).await.unwrap(), linux.id);

    delete_page(&db, install.id, cairn_core::UserId::new()).await.unwrap();
    let trash = list_trash(&db).await.unwrap();
    assert_eq!(trash.len(), 1, "the subpage is not listed on its own");
    assert_eq!((trash[0].page.title.as_str(), trash[0].subpages, trash[0].old_url.as_str()), ("Install", 1, "/guide/install/"));

    let (restored, kept) = restore_page(&db, install.id).await.unwrap();
    assert!(kept && restored.parent_id == Some(guide.id));
    assert_eq!(url(&list_pages(&db).await.unwrap(), linux.id), before);
    assert!(list_trash(&db).await.unwrap().is_empty());

    // When a new page took the URL meanwhile, the restored one gets a numbered segment instead of clashing.
    delete_page(&db, install.id, cairn_core::UserId::new()).await.unwrap();
    create_page(&db, user, Some(guide.id), "Install", Placement::Last).await.unwrap();
    let (again, kept) = restore_page(&db, install.id).await.unwrap();
    assert!(!kept && again.slug == "install-2");
    assert!(distinct_positions(list_pages(&db).await.unwrap()), "the new Install took the old position");

    // And when its parent is gone it returns to the top level.
    delete_page(&db, again.id, cairn_core::UserId::new()).await.unwrap();
    delete_page(&db, guide.id, cairn_core::UserId::new()).await.unwrap();
    create_page(&db, user, None, "About", Placement::Last).await.unwrap();
    let (top, kept) = restore_page(&db, again.id).await.unwrap();
    assert!(!kept && top.parent_id.is_none());
    assert!(distinct_positions(list_pages(&db).await.unwrap()), "every first child has the same position");
  }
}
