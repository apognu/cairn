use std::collections::HashMap;

use cairn_core::{Document, PageId, Redirect, RedirectOrigin, RevisionId};
use jiff::Timestamp;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set, sea_query::OnConflict};

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

/// Records a redirect to a fixed URL; one from the same path is replaced.
pub(crate) async fn redirect_to_url(db: &DatabaseConnection, from: &str, url: &str) -> Result<(), StoreError> {
  redirect::Entity::insert(redirect::ActiveModel {
    from_path: Set(from.to_string()),
    to_page_id: Set(None),
    to_url: Set(Some(url.to_string())),
    source: Set(RedirectOrigin::Manual.as_str().into()),
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

/// The saved document of every page that has a draft. A page nobody has edited has none, which publishes as an empty page.
pub async fn draft_docs(db: &DatabaseConnection) -> Result<HashMap<PageId, Document>, StoreError> {
  let rows = draft::Entity::find().all(db).await?;

  rows
    .into_iter()
    .map(|row| Ok((row.page_id.into(), serde_json::from_str(&row.doc).map_err(|e| StoreError::Invalid(format!("bad draft document: {e}")))?)))
    .collect()
}
