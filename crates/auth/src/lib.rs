//! Who is making a request: the OIDC login, and the `CurrentUser` extractor every writing handler takes.
//!
//! `yourdocs dev` runs every request as the built-in local user; `serve` requires a signed-in one.

#![forbid(unsafe_code)]

mod oidc;
mod routes;

use std::sync::Arc;

use axum::{
  Json, Router,
  extract::{FromRef, FromRequestParts},
  http::{StatusCode, request::Parts},
  response::{IntoResponse, Response},
};
use yourdocs_core::UserId;
use yourdocs_store::{DatabaseConnection, StoreError};

pub use crate::oidc::{Authorization, Identity, Oidc, OidcConfig};

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
      AuthError::Config(_) | AuthError::Store(_) | AuthError::Other(_) => {
        tracing::error!(error = %self, "authentication failed");
        (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
      }
    };

    // The same shape as the server's `ErrorBody`.
    (status, Json(serde_json::json!({ "message": message, "details": [] }))).into_response()
  }
}

/// How requests are authenticated; the server keeps one in its state.
#[derive(Clone)]
pub struct Auth(Arc<Mode>);

enum Mode {
  Local,
  Oidc(Box<OidcAuth>),
}

struct OidcAuth {
  oidc: Oidc,
  db: DatabaseConnection,
}

impl Auth {
  /// No login: everyone is the local user. Only for `dev`, which listens on localhost.
  pub fn local() -> Self {
    Self(Arc::new(Mode::Local))
  }

  /// Sign-in through the OpenID Connect provider.
  pub fn oidc(oidc: Oidc, db: DatabaseConnection) -> Self {
    Self(Arc::new(Mode::Oidc(Box::new(OidcAuth { oidc, db }))))
  }

  /// The login routes (`/auth/…`); empty without a provider.
  pub fn routes() -> Router<Auth> {
    routes::routes()
  }
}

/// The authenticated user of a request. Handlers that write take this, so how a request is authenticated changes in one place.
#[derive(Debug, Clone, Copy)]
pub struct CurrentUser(pub UserId);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser
where
  Auth: FromRef<S>,
{
  type Rejection = AuthError;

  async fn from_request_parts(_: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    match &*Auth::from_ref(state).0 {
      Mode::Local => Ok(CurrentUser(local_user_id())),
      // Sessions come with M5.2.
      Mode::Oidc(_) => Err(AuthError::Unauthorized),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn local_user_is_stable() {
    assert_eq!(local_user_id(), local_user_id());
  }
}
