use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use crate::{PageId, User, UserId};

/// Who looks after a page and when it should be checked again. Not content: it is saved at once and never published.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PageReview {
  pub owner_id: Option<UserId>,
  /// The day the page should be checked by, `YYYY-MM-DD`.
  #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = Date))]
  pub review_by: Option<Date>,
}

/// A page whose review date has come.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StalePage {
  pub page_id: PageId,
  pub title: String,
  /// `None` when nobody owns the page, or the owner's account is gone.
  pub owner: Option<User>,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = Date))]
  pub review_by: Date,
}
