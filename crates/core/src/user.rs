use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::UserId;

/// Someone who signed in through an identity provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct User {
  pub id: UserId,
  pub email: String,
  pub name: String,
}

/// A token for scripts and CI, as listed: the secret itself is shown only when the token is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiToken {
  #[cfg_attr(feature = "openapi", schema(value_type = String))]
  pub id: Uuid,
  pub name: String,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub created_at: Timestamp,
  #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = DateTime))]
  pub last_used_at: Option<Timestamp>,
}
