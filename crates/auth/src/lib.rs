//! Who is making a request. Until OIDC arrives (M5) every request runs as the built-in local user.

#![forbid(unsafe_code)]

use std::convert::Infallible;

use axum::{extract::FromRequestParts, http::request::Parts};
use yourdocs_core::UserId;

/// The user every request runs as in `yourdocs dev`. A fixed value so rows written in earlier sessions still point at it.
pub fn local_user_id() -> UserId {
  UserId::from(uuid::Uuid::from_u128(1))
}

/// The authenticated user of a request. Handlers that write take this, so swapping the allow-all layer for real auth touches one place.
#[derive(Debug, Clone, Copy)]
pub struct CurrentUser(pub UserId);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
  type Rejection = Infallible;

  async fn from_request_parts(_: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
    Ok(CurrentUser(local_user_id()))
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
