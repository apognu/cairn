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
  pub role: Role,
  /// None for `dev`'s user, who never signs in.
  #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = DateTime))]
  #[serde(default)]
  pub last_login_at: Option<Timestamp>,
}

/// What a user may do. Each role can do everything the ones before it can, so roles compare in that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum Role {
  /// Reads drafts, history and settings; changes nothing.
  Viewer,
  /// Edits pages, the library and redirects, and deploys previews.
  Editor,
  /// Publishes, promotes and rolls back.
  Publisher,
  /// Changes site settings and roles, and takes locks from people still at work.
  Admin,
}

impl Role {
  pub fn as_str(self) -> &'static str {
    match self {
      Role::Viewer => "viewer",
      Role::Editor => "editor",
      Role::Publisher => "publisher",
      Role::Admin => "admin",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    [Self::Viewer, Self::Editor, Self::Publisher, Self::Admin].into_iter().find(|r| r.as_str() == value)
  }
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
  /// The most the token may do; none means as much as its owner. It never does more than its owner can at the time.
  #[serde(default)]
  pub role: Option<Role>,
}
