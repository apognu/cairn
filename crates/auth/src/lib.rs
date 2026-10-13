//! Who is making a request: the OIDC login, and the `CurrentUser` extractor every writing handler takes.
//!
//! `yourdocs dev` runs every request as the built-in local user; `serve` requires a signed-in one.

#![forbid(unsafe_code)]

mod access;
mod oidc;
mod routes;
mod session;

use std::sync::Arc;

use axum::{
  Json, Router,
  extract::{FromRef, FromRequestParts, Request, State},
  http::{HeaderMap, StatusCode, request::Parts},
  middleware::Next,
  response::{IntoResponse, Response},
};
use yourdocs_core::UserId;
use yourdocs_store::{DatabaseConnection, StoreError, create_session, delete_session, session_user};

pub use crate::{
  access::AccessRules,
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
  sessions: SessionSettings,
  access: AccessRules,
}

impl Auth {
  /// No login: everyone is the local user. Only for `dev`, which listens on localhost.
  pub fn local() -> Self {
    Self(Arc::new(Mode::Local))
  }

  /// Sign-in through the OpenID Connect provider.
  pub fn oidc(oidc: Oidc, db: DatabaseConnection, sessions: SessionSettings, access: AccessRules) -> Self {
    Self(Arc::new(Mode::Oidc(Box::new(OidcAuth { oidc, db, sessions, access }))))
  }

  /// Who a request comes from: the local user without a login, otherwise the owner of its session cookie.
  pub async fn authenticate(&self, headers: &HeaderMap) -> Result<UserId, AuthError> {
    match &*self.0 {
      Mode::Local => Ok(local_user_id()),
      Mode::Oidc(oidc) => {
        let token = oidc.sessions.token(headers).ok_or(AuthError::Unauthorized)?;

        session_user(&oidc.db, &session::hash_token(token), oidc.sessions.idle).await?.ok_or(AuthError::Unauthorized)
      }
    }
  }

  /// Starts a session for a user who just signed in; returns the `Set-Cookie` value.
  pub(crate) async fn start_session(&self, user: UserId) -> Result<String, AuthError> {
    let Mode::Oidc(oidc) = &*self.0 else {
      return Err(AuthError::Login("this server has no login".into()));
    };
    let token = session::new_token();

    create_session(&oidc.db, &session::hash_token(&token), user, oidc.sessions.absolute).await?;

    Ok(oidc.sessions.set_cookie(&token))
  }

  /// Ends the session of a request, if it has one; returns the `Set-Cookie` value that removes the cookie.
  pub(crate) async fn end_session(&self, headers: &HeaderMap) -> Result<String, AuthError> {
    let Mode::Oidc(oidc) = &*self.0 else {
      return Err(AuthError::Login("this server has no login".into()));
    };

    if let Some(token) = oidc.sessions.token(headers) {
      delete_session(&oidc.db, &session::hash_token(token)).await?;
    }

    Ok(oidc.sessions.clear_cookie())
  }

  /// Whether requests need a login.
  pub fn requires_login(&self) -> bool {
    matches!(&*self.0, Mode::Oidc(_))
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

  async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    // The login guard already resolved it for routes behind it.
    if let Some(user) = parts.extensions.get::<CurrentUser>() {
      return Ok(*user);
    }

    Auth::from_ref(state).authenticate(&parts.headers).await.map(CurrentUser)
  }
}

/// Middleware: refuses requests without a valid login, so a route cannot be left open by a handler that forgot to take `CurrentUser`.
pub async fn require_login(State(auth): State<Auth>, mut request: Request, next: Next) -> Response {
  match auth.authenticate(request.headers()).await {
    Ok(user) => {
      request.extensions_mut().insert(CurrentUser(user));
      next.run(request).await
    }
    Err(error) => error.into_response(),
  }
}

#[cfg(test)]
mod tests {
  use axum::http::{HeaderValue, header};
  use jiff::SignedDuration;
  use yourdocs_store::{connect, upsert_user};

  use super::*;

  async fn signed_in(idle: SignedDuration) -> (Auth, HeaderMap) {
    let dir = std::env::temp_dir().join(format!("yourdocs-auth-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = connect(&dir.join("test.db")).await.unwrap();
    let user = upsert_user(&db, "https://idp.example", "alice", "alice@example.com", "Alice").await.unwrap();

    let auth = Auth::oidc(
      Oidc::unreachable(),
      db,
      SessionSettings {
        idle,
        absolute: SignedDuration::from_hours(24),
        secure: true,
      },
      AccessRules {
        allow_any: true,
        ..Default::default()
      },
    );
    let cookie = auth.start_session(user.id).await.unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, HeaderValue::from_str(cookie.split(';').next().unwrap()).unwrap());

    (auth, headers)
  }

  #[tokio::test]
  async fn a_session_authenticates_until_logout() {
    let (auth, headers) = signed_in(SignedDuration::from_hours(1)).await;

    assert!(auth.authenticate(&headers).await.is_ok());
    assert!(matches!(auth.authenticate(&HeaderMap::new()).await, Err(AuthError::Unauthorized)));

    auth.end_session(&headers).await.unwrap();

    assert!(matches!(auth.authenticate(&headers).await, Err(AuthError::Unauthorized)));
  }

  #[tokio::test]
  async fn an_idle_session_is_refused() {
    // A negative idle time is already exceeded when the session is created: no sleeping in a test.
    let (auth, headers) = signed_in(SignedDuration::from_secs(-1)).await;

    assert!(matches!(auth.authenticate(&headers).await, Err(AuthError::Unauthorized)));
  }

  #[test]
  fn local_user_is_stable() {
    assert_eq!(local_user_id(), local_user_id());
  }
}
