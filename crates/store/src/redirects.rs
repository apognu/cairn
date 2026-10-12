use std::collections::HashMap;

use jiff::Timestamp;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set, sea_query::OnConflict};
use yourdocs_core::{PageId, Redirect, RedirectOrigin, RevisionId};

use crate::{
  StoreError,
  entity::{draft, redirect},
};

pub async fn list_redirects(db: &DatabaseConnection) -> Result<Vec<Redirect>, StoreError> {
  let rows = redirect::Entity::find().order_by_asc(redirect::Column::FromPath).all(db).await?;

  rows
    .into_iter()
    .map(|row| {
      Ok(Redirect {
        from_path: row.from_path,
        to_page_id: row.to_page_id.map(Into::into),
        to_url: row.to_url,
        origin: RedirectOrigin::parse(&row.source).ok_or_else(|| StoreError::Invalid(format!("unknown redirect source {}", row.source)))?,
        created_at: row.created_at.parse().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")))?,
      })
    })
    .collect()
}

/// Records that `from` now leads to a page. An older redirect from the same path is replaced.
pub(crate) async fn redirect_to_page(db: &DatabaseConnection, from: &str, page: PageId, origin: RedirectOrigin) -> Result<(), StoreError> {
  redirect::Entity::insert(redirect::ActiveModel {
    from_path: Set(from.to_string()),
    to_page_id: Set(Some(page.as_uuid())),
    to_url: Set(None),
    source: Set(origin.as_str().into()),
    created_at: Set(Timestamp::now().to_string()),
  })
  .on_conflict(
    OnConflict::column(redirect::Column::FromPath)
      .update_columns([redirect::Column::ToPageId, redirect::Column::ToUrl, redirect::Column::Source, redirect::Column::CreatedAt])
      .to_owned(),
  )
  .exec(db)
  .await?;

  Ok(())
}

/// Drops the redirects that start at paths a page now occupies, so a page moved back never redirects to itself.
pub(crate) async fn forget_redirects_from(db: &DatabaseConnection, paths: Vec<String>) -> Result<(), StoreError> {
  if !paths.is_empty() {
    redirect::Entity::delete_many().filter(redirect::Column::FromPath.is_in(paths)).exec(db).await?;
  }

  Ok(())
}

/// The latest saved revision of every page that has a draft.
pub async fn draft_revisions(db: &DatabaseConnection) -> Result<HashMap<PageId, RevisionId>, StoreError> {
  let rows = draft::Entity::find().all(db).await?;

  Ok(rows.into_iter().filter_map(|row| Some((row.page_id.into(), row.base_revision_id?.into()))).collect())
}
