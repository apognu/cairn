use cairn_core::{LinkKind, PageId, PageLink};
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::{
  StoreError,
  entity::{page, page_link},
};

/// Replaces what a page links to with what its document links to now.
pub(crate) async fn replace_links(db: &impl ConnectionTrait, source: PageId, links: &[PageLink]) -> Result<(), StoreError> {
  page_link::Entity::delete_many().filter(page_link::Column::SourcePageId.eq(source.as_uuid())).exec(db).await?;

  if links.is_empty() {
    return Ok(());
  }

  // A link to a page that does not exist (a stale ID pasted in) would break the foreign key; it is simply not recorded.
  let known: Vec<_> = page::Entity::find()
    .filter(page::Column::Id.is_in(links.iter().map(|l| l.target.as_uuid())))
    .all(db)
    .await?
    .into_iter()
    .map(|p| p.id)
    .collect();
  let rows = links
    .iter()
    .filter(|l| l.target != source && known.contains(&l.target.as_uuid()))
    .map(|l| page_link::ActiveModel {
      source_page_id: Set(source.as_uuid()),
      target_page_id: Set(l.target.as_uuid()),
      anchor: Set(l.anchor.clone()),
      kind: Set(l.kind.as_str().into()),
    })
    .collect::<Vec<_>>();

  if !rows.is_empty() {
    page_link::Entity::insert_many(rows).on_conflict_do_nothing().exec_without_returning(db).await?;
  }

  Ok(())
}

/// One recorded reference: `source` links to `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredLink {
  pub source: PageId,
  pub target: PageId,
  pub kind: LinkKind,
}

/// Every recorded link. Filtering by live pages is left to the caller, which has the page list.
pub async fn all_links(db: &DatabaseConnection) -> Result<Vec<StoredLink>, StoreError> {
  let rows = page_link::Entity::find().all(db).await?;

  Ok(
    rows
      .into_iter()
      .filter_map(|r| {
        Some(StoredLink {
          source: r.source_page_id.into(),
          target: r.target_page_id.into(),
          kind: LinkKind::parse(&r.kind)?,
        })
      })
      .collect(),
  )
}
