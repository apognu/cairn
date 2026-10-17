//! Page owners and review dates.

use cairn_core::{PageId, PageReview, User, UserId};
use jiff::{Timestamp, civil::Date};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set, sea_query::OnConflict};

use crate::{
  StoreError,
  entity::{page, page_review, user},
  users::into_user,
};

pub async fn page_review(db: &DatabaseConnection, page: PageId) -> Result<PageReview, StoreError> {
  let row = page_review::Entity::find_by_id(page.as_uuid()).one(db).await?;

  row.map_or(Ok(PageReview::default()), into_review)
}

/// Sets a page's owner and review date, both at once; `None` clears them.
pub async fn set_page_review(db: &DatabaseConnection, page: PageId, review: &PageReview, by: UserId) -> Result<PageReview, StoreError> {
  page::Entity::find_by_id(page.as_uuid())
    .filter(page::Column::DeletedAt.is_null())
    .one(db)
    .await?
    .ok_or(StoreError::NotFound)?;

  let row = page_review::ActiveModel {
    page_id: Set(page.as_uuid()),
    owner_id: Set(review.owner_id.map(|id| id.as_uuid())),
    review_by: Set(review.review_by.map(|date| date.to_string())),
    updated_by: Set(by.as_uuid()),
    updated_at: Set(Timestamp::now().to_string()),
  };
  page_review::Entity::insert(row)
    .on_conflict(
      OnConflict::column(page_review::Column::PageId)
        .update_columns([
          page_review::Column::OwnerId,
          page_review::Column::ReviewBy,
          page_review::Column::UpdatedBy,
          page_review::Column::UpdatedAt,
        ])
        .to_owned(),
    )
    .exec(db)
    .await?;

  Ok(review.clone())
}

/// A page whose review date has come, with the ID of its owner.
pub struct StaleRow {
  pub page_id: PageId,
  pub title: String,
  pub owner_id: Option<UserId>,
  pub review_by: Date,
}

/// The pages in the tree (not in the trash) whose review date is `today` or earlier, the most overdue first.
pub async fn stale_pages(db: &DatabaseConnection, today: Date) -> Result<Vec<StaleRow>, StoreError> {
  let reviews = page_review::Entity::find().filter(page_review::Column::ReviewBy.is_not_null()).all(db).await?;
  let pages = page::Entity::find().filter(page::Column::DeletedAt.is_null()).all(db).await?;

  let mut stale: Vec<StaleRow> = reviews
    .into_iter()
    .filter_map(|row| {
      let review_by: Date = row.review_by.as_deref()?.parse().ok()?;
      let page = pages.iter().find(|p| p.id == row.page_id)?;
      (review_by <= today).then(|| StaleRow {
        page_id: page.id.into(),
        title: page.title.clone(),
        owner_id: row.owner_id.map(Into::into),
        review_by,
      })
    })
    .collect();
  stale.sort_by(|a, b| a.review_by.cmp(&b.review_by).then_with(|| a.title.cmp(&b.title)));

  Ok(stale)
}

/// Everyone who ever signed in, by name.
pub async fn list_users(db: &DatabaseConnection) -> Result<Vec<User>, StoreError> {
  let rows = user::Entity::find().order_by_asc(user::Column::Name).all(db).await?;

  rows.into_iter().map(into_user).collect()
}

fn into_review(row: page_review::Model) -> Result<PageReview, StoreError> {
  Ok(PageReview {
    owner_id: row.owner_id.map(Into::into),
    review_by: row.review_by.as_deref().map(str::parse).transpose().map_err(|e| StoreError::Invalid(format!("bad review date: {e}")))?,
  })
}
