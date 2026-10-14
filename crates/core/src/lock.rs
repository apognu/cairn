use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::{PageId, UserId};

/// How long a lock lasts without a renewal. The editor renews it every 30 seconds while its author is active, so a closed tab frees the
/// page within this time.
pub const LOCK_LEASE: SignedDuration = SignedDuration::from_mins(2);

/// Who is editing a page, and since when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PageLock {
  pub page_id: PageId,
  pub user_id: UserId,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub acquired_at: Timestamp,
  /// The last renewal: the holder's editor was open and in use then.
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub renewed_at: Timestamp,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub expires_at: Timestamp,
}
