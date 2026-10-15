use std::collections::{HashMap, HashSet};

use cairn_core::{Page, PageId, Release, ReleaseStatus};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::{
  StoreError,
  entity::{release, release_page},
  releases::into_release,
};

/// The releases listed at most; older ones are not offered for rollback.
const LOGGED: u64 = 50;

/// A release that was, or is, the site, with what it changed compared with the release it was built on.
pub struct ReleaseEntry {
  pub release: Release,
  /// 1 for the first release that went live, counting every later one: releases are never deleted, so a number stays with its release.
  pub number: u32,
  /// Titles of the pages whose revision differs from the base's, that are new, or that the base had and this one does not.
  pub changes: Vec<String>,
}

/// The latest releases that were live, newest first.
pub async fn release_log(db: &impl ConnectionTrait) -> Result<Vec<ReleaseEntry>, StoreError> {
  let went_live = [ReleaseStatus::Live.as_str(), ReleaseStatus::Superseded.as_str()];
  let ids: Vec<Uuid> = release::Entity::find()
    .filter(release::Column::Status.is_in(went_live))
    .order_by_asc(release::Column::Id)
    .select_only()
    .column(release::Column::Id)
    .into_tuple()
    .all(db)
    .await?;
  let rows = release::Entity::find()
    .filter(release::Column::Status.is_in(went_live))
    .order_by_desc(release::Column::Id)
    .limit(LOGGED)
    .all(db)
    .await?;

  // The bases are usually among the rows, but the oldest row's is not; their trees name the pages a release removed.
  let wanted: HashSet<Uuid> = rows.iter().flat_map(|r| [Some(r.id), r.base_release_id]).flatten().collect();
  let revisions = revisions_of(db, &wanted).await?;
  let mut trees: HashMap<Uuid, Vec<Page>> = rows.iter().map(|r| (r.id, parse_tree(&r.tree_snapshot))).collect();
  let missing: Vec<Uuid> = rows.iter().filter_map(|r| r.base_release_id).filter(|id| !trees.contains_key(id)).collect();
  for base in release::Entity::find().filter(release::Column::Id.is_in(missing)).all(db).await? {
    trees.insert(base.id, parse_tree(&base.tree_snapshot));
  }

  let mut log = Vec::with_capacity(rows.len());
  for row in rows {
    let number = ids.iter().position(|id| *id == row.id).map_or(0, |i| i as u32 + 1);
    let none = HashMap::new();
    let before = row.base_release_id.and_then(|b| revisions.get(&b)).unwrap_or(&none);
    let after = revisions.get(&row.id).unwrap_or(&none);

    let mut changes = Vec::new();
    for page in trees.get(&row.id).into_iter().flatten().filter(|p| after.contains_key(&p.id)) {
      if before.get(&page.id) != after.get(&page.id) {
        changes.push(page.title.clone());
      }
    }
    for page in row
      .base_release_id
      .and_then(|b| trees.get(&b))
      .into_iter()
      .flatten()
      .filter(|p| before.contains_key(&p.id) && !after.contains_key(&p.id))
    {
      changes.push(page.title.clone());
    }

    log.push(ReleaseEntry {
      release: into_release(db, row).await?,
      number,
      changes,
    });
  }

  Ok(log)
}

fn parse_tree(json: &str) -> Vec<Page> {
  serde_json::from_str(json).unwrap_or_default()
}

/// Each release's pages and the revision it published of them.
async fn revisions_of(db: &impl ConnectionTrait, releases: &HashSet<Uuid>) -> Result<HashMap<Uuid, HashMap<PageId, Uuid>>, StoreError> {
  let rows = release_page::Entity::find().filter(release_page::Column::ReleaseId.is_in(releases.iter().copied())).all(db).await?;
  let mut found: HashMap<Uuid, HashMap<PageId, Uuid>> = HashMap::new();
  for row in rows {
    found.entry(row.release_id).or_default().insert(row.page_id.into(), row.revision_id);
  }

  Ok(found)
}
