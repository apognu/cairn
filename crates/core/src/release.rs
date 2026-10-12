use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{ReleaseId, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStatus {
  Building,
  Live,
  Failed,
}

impl ReleaseStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      ReleaseStatus::Building => "building",
      ReleaseStatus::Live => "live",
      ReleaseStatus::Failed => "failed",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    [Self::Building, Self::Live, Self::Failed].into_iter().find(|s| s.as_str() == value)
  }
}

/// An immutable published state of the whole site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Release {
  pub id: ReleaseId,
  /// The live release this one was built on.
  pub base_release_id: Option<ReleaseId>,
  pub created_by: UserId,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub created_at: Timestamp,
  pub status: ReleaseStatus,
  /// How many pages the release contains.
  pub page_count: u32,
}
