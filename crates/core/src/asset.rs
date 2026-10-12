use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{AssetId, UserId};

/// An uploaded file. The bytes live in the `AssetStore` under the content hash; this is the row that describes them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Asset {
  pub id: AssetId,
  /// Hex SHA-256 of the file: two uploads of the same bytes are one asset.
  pub sha256: String,
  pub mime: String,
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub bytes: u64,
  pub uploaded_by: UserId,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub created_at: Timestamp,
}
