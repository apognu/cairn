use serde::{Deserialize, Serialize};

use crate::UserId;

/// Someone who signed in through an identity provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct User {
  pub id: UserId,
  pub email: String,
  pub name: String,
}
