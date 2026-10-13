//! Who is making a request: the OIDC login, and the `CurrentUser` extractor every writing handler takes.
//!
//! `yourdocs dev` runs every request as the built-in local user; `serve` requires a signed-in one.

#![forbid(unsafe_code)]

mod access;
mod auth;
mod github;
mod oidc;
mod routes;
mod session;

use axum::{
  Json,
  http::StatusCode,
  response::{IntoResponse, Response},
};
use yourdocs_core::UserId;
use yourdocs_store::StoreError;

pub use crate::{
  access::AccessRules,
  auth::{Auth, AuthSettings, Credential, CurrentUser, InteractiveUser, require_login},
  github::{GITHUB_ISSUER, Github, GithubConfig},
  oidc::{Authorization, Identity, Oidc, OidcConfig},
  session::SessionSettings,
};

/// The user every request runs as in `yourdocs dev`. A fixed value so rows written in earlier sessions still point at it.
pub fn local_user_id() -> UserId {
  UserId::from(uuid::Uuid::from_u128(1))
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
  /// The deployment's authentication settings cannot work; found at startup.
  #[error("{0}")]
  Config(String),
  /// The provider's answer to a login was refused.
  #[error("{0}")]
  Login(String),
  #[error("sign in to continue")]
  Unauthorized,
  /// Signed in, but not allowed to do this.
  #[error("{0}")]
  Forbidden(String),
  #[error(transparent)]
  Store(#[from] StoreError),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}

impl IntoResponse for AuthError {
  fn into_response(self) -> Response {
    let (status, message) = match &self {
      AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
      AuthError::Login(_) => (StatusCode::BAD_REQUEST, self.to_string()),
      AuthError::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),
      AuthError::Config(_) | AuthError::Store(_) | AuthError::Other(_) => {
        tracing::error!(error = %self, "authentication failed");
        (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
      }
    };

    // The same shape as the server's `ErrorBody`.
    (status, Json(serde_json::json!({ "message": message, "details": [] }))).into_response()
  }
}
