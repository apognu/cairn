//! The configured way of authenticating requests, and the extractors and middleware built on it.

use std::sync::Arc;

use axum::{
  Router,
  extract::{FromRef, FromRequestParts, Request, State},
  http::{HeaderMap, Method, header, request::Parts},
  middleware::Next,
  response::{IntoResponse, Response},
};
use yourdocs_core::{ApiToken, UserId};
use yourdocs_store::{DatabaseConnection, api_token_user, create_api_token, create_session, delete_session, session_user};

use crate::{AccessRules, AuthError, Oidc, SessionSettings, local_user_id, routes, session};

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
  settings: AuthSettings,
}

/// What a deployment with sign-in decides.
#[derive(Debug, Clone)]
pub struct AuthSettings {
  pub sessions: SessionSettings,
  pub access: AccessRules,
  /// The origin editors use (`https://docs.example.com`): requests that change something must come from it.
  pub public_origin: String,
}

/// What proved who a request is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credential {
  /// `dev`, which has no login.
  Local,
  /// A browser session cookie, which the browser sends by itself: the reason changes are checked for their origin.
  Session,
  /// An API token in the `Authorization` header, which only a script that has it can send.
  Token,
}

/// The prefix of every API token, so a leaked one is recognizable (and searchable by secret scanners).
const TOKEN_PREFIX: &str = "yd_";

impl Auth {
  /// No login: everyone is the local user. Only for `dev`, which listens on localhost.
  pub fn local() -> Self {
    Self(Arc::new(Mode::Local))
  }

  /// Sign-in through the OpenID Connect provider.
  pub fn oidc(oidc: Oidc, db: DatabaseConnection, settings: AuthSettings) -> Self {
    Self(Arc::new(Mode::Oidc(Box::new(OidcAuth { oidc, db, settings }))))
  }

  /// Who a request comes from: the local user without a login; otherwise the owner of its API token, or of its session cookie.
  pub async fn authenticate(&self, headers: &HeaderMap) -> Result<(UserId, Credential), AuthError> {
    let Mode::Oidc(oidc) = &*self.0 else { return Ok((local_user_id(), Credential::Local)) };

    // A request that carries a token is judged by the token alone: a bad one is not rescued by a cookie.
    if let Some(value) = headers.get(header::AUTHORIZATION) {
      let secret = value.to_str().ok().and_then(|v| v.strip_prefix("Bearer ")).ok_or(AuthError::Unauthorized)?;
      let user = api_token_user(&oidc.db, &session::hash_token(secret.trim())).await?.ok_or(AuthError::Unauthorized)?;

      return Ok((user, Credential::Token));
    }

    let token = oidc.settings.sessions.token(headers).ok_or(AuthError::Unauthorized)?;
    let user = session_user(&oidc.db, &session::hash_token(token), oidc.settings.sessions.idle).await?.ok_or(AuthError::Unauthorized)?;

    Ok((user, Credential::Session))
  }

  /// Refuses a request that changes something unless it comes from the editor's own origin. Browsers send the `Origin` of every request
  /// that is not a plain read; a page on another site cannot forge it, and without a cookie to ride on a script has nothing to abuse.
  pub fn check_origin(&self, method: &Method, headers: &HeaderMap) -> Result<(), AuthError> {
    let Mode::Oidc(oidc) = &*self.0 else { return Ok(()) };
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
      return Ok(());
    }

    let allowed = oidc.settings.public_origin.as_str();
    let text = |name| headers.get(name).and_then(|v| v.to_str().ok());
    let same = match text(header::ORIGIN) {
      Some(origin) => origin == allowed,
      // Some browsers leave Origin out of same-origin requests of older kinds; the Referer says the same.
      None => text(header::REFERER).is_some_and(|referer| referer == allowed || referer.strip_prefix(allowed).is_some_and(|rest| rest.starts_with('/'))),
    };

    same.then_some(()).ok_or_else(|| AuthError::Forbidden("this request does not come from the editor".into()))
  }

  /// Starts a session for a user who just signed in; returns the `Set-Cookie` value.
  pub(crate) async fn start_session(&self, user: UserId) -> Result<String, AuthError> {
    let Mode::Oidc(oidc) = &*self.0 else {
      return Err(AuthError::Login("this server has no login".into()));
    };
    let token = session::new_token();

    create_session(&oidc.db, &session::hash_token(&token), user, oidc.settings.sessions.absolute).await?;

    Ok(oidc.settings.sessions.set_cookie(&token))
  }

  /// Ends the session of a request, if it has one; returns the `Set-Cookie` value that removes the cookie.
  pub(crate) async fn end_session(&self, headers: &HeaderMap) -> Result<String, AuthError> {
    let Mode::Oidc(oidc) = &*self.0 else {
      return Err(AuthError::Login("this server has no login".into()));
    };

    if let Some(token) = oidc.settings.sessions.token(headers) {
      delete_session(&oidc.db, &session::hash_token(token)).await?;
    }

    Ok(oidc.settings.sessions.clear_cookie())
  }

  /// Makes an API token for a user. The secret is returned here and nowhere else: only its hash is stored.
  pub async fn issue_token(&self, user: UserId, name: &str) -> Result<(ApiToken, String), AuthError> {
    let Mode::Oidc(oidc) = &*self.0 else {
      return Err(AuthError::Forbidden("API tokens belong to signed-in users; `dev` has none".into()));
    };
    let secret = format!("{TOKEN_PREFIX}{}", session::new_token());
    let token = create_api_token(&oidc.db, user, name, &session::hash_token(&secret)).await?;

    Ok((token, secret))
  }

  /// The login routes (`/auth/…`).
  pub fn routes() -> Router<Auth> {
    routes::routes()
  }

  pub(crate) fn oidc_parts(&self) -> Option<(&Oidc, &DatabaseConnection, &AccessRules)> {
    match &*self.0 {
      Mode::Oidc(oidc) => Some((&oidc.oidc, &oidc.db, &oidc.settings.access)),
      Mode::Local => None,
    }
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

    Auth::from_ref(state).authenticate(&parts.headers).await.map(|(user, _)| CurrentUser(user))
  }
}

/// A user signed in with a browser session, not an API token: for what a script must not do, such as making more tokens.
#[derive(Debug, Clone, Copy)]
pub struct InteractiveUser(pub UserId);

impl<S: Send + Sync> FromRequestParts<S> for InteractiveUser
where
  Auth: FromRef<S>,
{
  type Rejection = AuthError;

  async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    let (user, credential) = match (parts.extensions.get::<CurrentUser>(), parts.extensions.get::<Credential>()) {
      (Some(user), Some(credential)) => (user.0, *credential),
      _ => Auth::from_ref(state).authenticate(&parts.headers).await?,
    };

    match credential {
      Credential::Token => Err(AuthError::Forbidden("this needs a signed-in session, not an API token".into())),
      Credential::Local | Credential::Session => Ok(InteractiveUser(user)),
    }
  }
}

/// Middleware: refuses requests without a valid login, so a route cannot be left open by a handler that forgot to take `CurrentUser`.
/// Requests that rely on the session cookie must also come from the editor's origin.
pub async fn require_login(State(auth): State<Auth>, mut request: Request, next: Next) -> Response {
  let (user, credential) = match auth.authenticate(request.headers()).await {
    Ok(found) => found,
    Err(error) => return error.into_response(),
  };

  if credential == Credential::Session
    && let Err(error) = auth.check_origin(request.method(), request.headers())
  {
    return error.into_response();
  }

  request.extensions_mut().insert(CurrentUser(user));
  request.extensions_mut().insert(credential);

  next.run(request).await
}

#[cfg(test)]
mod tests {
  use axum::http::HeaderValue;
  use jiff::SignedDuration;
  use yourdocs_store::{connect, delete_api_token, list_api_tokens, upsert_user};

  use super::*;

  const ORIGIN: &str = "https://docs.example.com";

  async fn signed_in(idle: SignedDuration) -> (Auth, UserId, HeaderMap) {
    let dir = std::env::temp_dir().join(format!("yourdocs-auth-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = connect(&dir.join("test.db")).await.unwrap();
    let user = upsert_user(&db, "https://idp.example", "alice", "alice@example.com", "Alice").await.unwrap();

    let auth = Auth::oidc(
      Oidc::unreachable(),
      db,
      AuthSettings {
        sessions: SessionSettings {
          idle,
          absolute: SignedDuration::from_hours(24),
          secure: true,
        },
        access: AccessRules {
          allow_any: true,
          ..Default::default()
        },
        public_origin: ORIGIN.into(),
      },
    );
    let cookie = auth.start_session(user.id).await.unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, HeaderValue::from_str(cookie.split(';').next().unwrap()).unwrap());

    (auth, user.id, headers)
  }

  #[tokio::test]
  async fn a_session_authenticates_until_logout() {
    let (auth, user, headers) = signed_in(SignedDuration::from_hours(1)).await;

    assert_eq!(auth.authenticate(&headers).await.unwrap(), (user, Credential::Session));
    assert!(matches!(auth.authenticate(&HeaderMap::new()).await, Err(AuthError::Unauthorized)));

    auth.end_session(&headers).await.unwrap();

    assert!(matches!(auth.authenticate(&headers).await, Err(AuthError::Unauthorized)));
  }

  #[tokio::test]
  async fn an_idle_session_is_refused() {
    // A negative idle time is already exceeded when the session is created: no sleeping in a test.
    let (auth, _, headers) = signed_in(SignedDuration::from_secs(-1)).await;

    assert!(matches!(auth.authenticate(&headers).await, Err(AuthError::Unauthorized)));
  }

  #[tokio::test]
  async fn changes_must_come_from_the_public_origin() {
    let (auth, _, _) = signed_in(SignedDuration::from_hours(1)).await;

    // (method, Origin, Referer, allowed)
    for (method, origin, referer, allowed) in [
      (Method::POST, Some(ORIGIN), None, true),
      (Method::POST, Some("https://evil.example"), None, false),
      (Method::POST, Some("null"), None, false),
      (Method::POST, Some("https://docs.example.com.evil.example"), None, false),
      (Method::POST, None, Some("https://docs.example.com/_editor/"), true),
      (Method::POST, None, Some("https://docs.example.com.evil.example/x"), false),
      (Method::DELETE, None, None, false),
      (Method::GET, Some("https://evil.example"), None, true),
      (Method::GET, None, None, true),
    ] {
      let mut headers = HeaderMap::new();
      for (name, value) in [(header::ORIGIN, origin), (header::REFERER, referer)] {
        if let Some(value) = value {
          headers.insert(name, HeaderValue::from_str(value).unwrap());
        }
      }

      assert_eq!(auth.check_origin(&method, &headers).is_ok(), allowed, "{method} {origin:?} {referer:?}");
    }
  }

  #[tokio::test]
  async fn a_token_authenticates_as_its_owner_until_deleted() {
    let (auth, user, cookie) = signed_in(SignedDuration::from_hours(1)).await;
    let (token, secret) = auth.issue_token(user, "CI").await.unwrap();
    let bearer = |value: &str| {
      let mut headers = cookie.clone();
      headers.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
      headers
    };

    assert!(secret.starts_with("yd_"));
    assert_eq!(auth.authenticate(&bearer(&format!("Bearer {secret}"))).await.unwrap(), (user, Credential::Token));
    // A wrong token is not rescued by the valid cookie that comes with it.
    assert!(matches!(auth.authenticate(&bearer("Bearer yd_wrong")).await, Err(AuthError::Unauthorized)));
    assert!(matches!(auth.authenticate(&bearer(&secret)).await, Err(AuthError::Unauthorized)));

    let Mode::Oidc(oidc) = &*auth.0 else { unreachable!() };
    assert_eq!(list_api_tokens(&oidc.db, user).await.unwrap().len(), 1);
    delete_api_token(&oidc.db, user, token.id).await.unwrap();

    assert!(matches!(auth.authenticate(&bearer(&format!("Bearer {secret}"))).await, Err(AuthError::Unauthorized)));
  }

  #[tokio::test]
  async fn the_guard_refuses_foreign_origins_for_cookies_but_not_for_tokens() {
    use axum::{body::Body, http::Request, middleware, routing::post};
    use tower::ServiceExt;

    let (auth, user, cookie) = signed_in(SignedDuration::from_hours(1)).await;
    let (_, secret) = auth.issue_token(user, "CI").await.unwrap();
    let app = axum::Router::new()
      .route("/api/publish", post(|CurrentUser(_): CurrentUser| async { "published" }))
      .layer(middleware::from_fn_with_state(auth.clone(), require_login))
      .with_state(auth.clone());

    let send = |origin: Option<&str>, bearer: Option<&str>, with_cookie: bool| {
      let mut request = Request::post("/api/publish");
      if let Some(origin) = origin {
        request = request.header(header::ORIGIN, origin);
      }
      if let Some(secret) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {secret}"));
      }
      if with_cookie {
        request = request.header(header::COOKIE, cookie[header::COOKIE].clone());
      }

      app.clone().oneshot(request.body(Body::empty()).unwrap())
    };

    // (Origin, token, cookie, expected status)
    for (origin, bearer, with_cookie, expected) in [
      (Some("https://evil.example"), None, true, 403),
      (None, None, true, 403),
      (Some(ORIGIN), None, true, 200),
      (None, Some(secret.as_str()), false, 200),
      (Some("https://evil.example"), Some(secret.as_str()), false, 200),
      (Some(ORIGIN), None, false, 401),
    ] {
      let status = send(origin, bearer, with_cookie).await.unwrap().status().as_u16();

      assert_eq!(status, expected, "{origin:?} token={} cookie={with_cookie}", bearer.is_some());
    }
  }
}
