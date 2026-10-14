use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{ReleaseId, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStatus {
  Building,
  /// Built and handed to the target, but not the site: it can be looked at and then promoted.
  Preview,
  /// The site as it is now. At most one release is live.
  Live,
  /// Was live; a later release replaced it.
  Superseded,
  Failed,
}

impl ReleaseStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      ReleaseStatus::Building => "building",
      ReleaseStatus::Preview => "preview",
      ReleaseStatus::Live => "live",
      ReleaseStatus::Superseded => "superseded",
      ReleaseStatus::Failed => "failed",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    [Self::Building, Self::Preview, Self::Live, Self::Superseded, Self::Failed].into_iter().find(|s| s.as_str() == value)
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
  /// Where a preview can be looked at, when its target gives it an address of its own.
  #[serde(default)]
  pub preview_url: Option<String>,
  /// How many files the release sent to its target, and their total size; unknown for releases made before they were recorded.
  #[serde(default)]
  pub file_count: Option<u64>,
  #[serde(default)]
  pub bytes: Option<u64>,
}
