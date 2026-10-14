use axum::{
  Router,
  extract::{Query, State},
  http::{HeaderMap, Method, StatusCode, header},
  response::{AppendHeaders, Html, IntoResponse, Redirect, Response},
  routing::{get, post},
};
use cairn_store::{LoginState, save_login_state, take_login_state, upsert_user};
use serde::Deserialize;

use crate::{Auth, AuthError};

pub(crate) fn routes() -> Router<Auth> {
  Router::new()
    .route("/auth/login", get(login))
    .route("/auth/login/oidc", get(login_oidc))
    .route("/auth/login/github", get(login_github))
    .route("/auth/callback", get(callback_oidc))
    .route("/auth/github/callback", get(callback_github))
    .route("/auth/logout", post(logout))
    .route("/auth/signed-out", get(signed_out))
}

#[derive(Deserialize)]
struct LoginQuery {
  next: Option<String>,
}

/// With one provider the login starts at once; with GitHub as well, the person chooses.
async fn login(State(auth): State<Auth>, Query(query): Query<LoginQuery>) -> Result<Response, AuthError> {
  let parts = auth.parts().ok_or_else(no_login)?;
  if parts.github.is_none() {
    return start(&auth, Provider::Oidc, query.next.as_deref()).await;
  }

  let base = auth.base_path();
  let next = query
    .next
    .as_deref()
    .map(|n| format!("?next={}", percent_encoding::utf8_percent_encode(n, percent_encoding::NON_ALPHANUMERIC)))
    .unwrap_or_default();
  let host = parts.settings.public_origin.split("://").nth(1).unwrap_or_default();

  Ok(Html(format!(
    "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width, initial-scale=1\"><meta name=color-scheme content=\"light dark\"><title>Sign in</title><p>Sign in to {host}</p><p><a href=\"{base}/auth/login/oidc{next}\">Sign in with single sign-on</a></p><p><a href=\"{base}/auth/login/github{next}\">Sign in with GitHub</a></p>",
    host = escape(host)
  ))
  .into_response())
}

async fn login_oidc(State(auth): State<Auth>, Query(query): Query<LoginQuery>) -> Result<Response, AuthError> {
  start(&auth, Provider::Oidc, query.next.as_deref()).await
}

async fn login_github(State(auth): State<Auth>, Query(query): Query<LoginQuery>) -> Result<Response, AuthError> {
  start(&auth, Provider::Github, query.next.as_deref()).await
}

#[derive(Clone, Copy)]
enum Provider {
  Oidc,
  Github,
}

fn no_login() -> AuthError {
  AuthError::Login("this server has no login".into())
}

/// Starts a login: remembers the PKCE verifier and nonce under the state value, and the state in the browser, and sends the browser to the
/// provider.
async fn start(auth: &Auth, provider: Provider, next: Option<&str>) -> Result<Response, AuthError> {
  let parts = auth.parts().ok_or_else(no_login)?;
  let authorization = match provider {
    Provider::Oidc => parts.oidc.authorize(),
    Provider::Github => parts.github.as_ref().ok_or_else(|| AuthError::Login("sign-in with GitHub is not set up".into()))?.authorize(),
  };

  let login = LoginState {
    verifier: authorization.verifier,
    nonce: authorization.nonce,
    next: local_path(next, auth.base_path()),
  };
  save_login_state(&parts.db, &authorization.state, &login).await?;
  let cookie = parts.settings.sessions.set_login_cookie(&authorization.state);

  Ok(([(header::SET_COOKIE, cookie)], Redirect::to(&authorization.url)).into_response())
}

#[derive(Deserialize)]
struct CallbackQuery {
  code: Option<String>,
  state: Option<String>,
  error: Option<String>,
  error_description: Option<String>,
}

async fn callback_oidc(State(auth): State<Auth>, headers: HeaderMap, Query(query): Query<CallbackQuery>) -> Response {
  callback(&auth, Provider::Oidc, &headers, query).await
}

async fn callback_github(State(auth): State<Auth>, headers: HeaderMap, Query(query): Query<CallbackQuery>) -> Response {
  callback(&auth, Provider::Github, &headers, query).await
}

/// The provider sends the browser back here with a code. Failures are shown as a page, since a person is looking at it.
async fn callback(auth: &Auth, provider: Provider, headers: &HeaderMap, query: CallbackQuery) -> Response {
  // The login is over either way: its cookie goes.
  let clear = auth.parts().map(|parts| parts.settings.sessions.clear_login_cookie()).unwrap_or_default();

  match finish_login(auth, provider, headers, query).await {
    Ok((cookie, next)) => (AppendHeaders([(header::SET_COOKIE, clear), (header::SET_COOKIE, cookie)]), Redirect::to(&next)).into_response(),
    Err(error) => {
      let message = match &error {
        AuthError::Login(message) => message.clone(),
        other => {
          tracing::error!(error = %other, "login failed");
          "The sign-in could not be completed. Try again.".to_string()
        }
      };
      (StatusCode::BAD_REQUEST, [(header::SET_COOKIE, clear)], Html(error_page(&message, auth.base_path()))).into_response()
    }
  }
}

/// Completes a login; returns the cookie that starts the session and where to send the browser.
async fn finish_login(auth: &Auth, provider: Provider, headers: &HeaderMap, query: CallbackQuery) -> Result<(String, String), AuthError> {
  let parts = auth.parts().ok_or_else(no_login)?;

  if let Some(error) = query.error {
    return Err(AuthError::Login(format!("the provider refused the sign-in: {}", query.error_description.unwrap_or(error))));
  }
  let (Some(code), Some(state)) = (query.code, query.state) else {
    return Err(AuthError::Login("the sign-in answer is incomplete".into()));
  };
  // Only the browser that started this login may finish it.
  if parts.settings.sessions.login_state(headers) != Some(state.as_str()) {
    return Err(AuthError::Login("this sign-in was not started in this browser; start again".into()));
  }
  let login = take_login_state(&parts.db, &state)
    .await?
    .ok_or_else(|| AuthError::Login("this sign-in expired or was already used; start again".into()))?;

  let (identity, allowed) = match provider {
    Provider::Oidc => {
      let identity = parts.oidc.complete(code, login.verifier, login.nonce).await?;
      let allowed = parts.settings.access.allows(&identity);
      (identity, allowed)
    }
    Provider::Github => {
      let github = parts.github.as_ref().ok_or_else(|| AuthError::Login("sign-in with GitHub is not set up".into()))?;
      let identity = github.complete(&code, &login.verifier).await?;
      let allowed = github.allows(&identity);
      (identity, allowed)
    }
  };

  // Checked before the user is stored: someone refused leaves no trace.
  if !allowed {
    tracing::warn!(issuer = %identity.issuer, subject = %identity.subject, "sign-in refused: not in an allowed group or domain");
    return Err(AuthError::Login(
      "Your account is not allowed to use this site. Ask an administrator to add you to an allowed group.".into(),
    ));
  }
  let user = upsert_user(&parts.db, &identity.issuer, &identity.subject, &identity.email, &identity.name).await?;
  tracing::info!(user_id = %user.id, issuer = %identity.issuer, "signed in");

  let cookie = auth.start_session(user.id).await?;

  Ok((cookie, login.next))
}

/// Where the editor goes after signing out. Not straight back to the provider, which may still have its own session open and would
/// sign the person in again at once.
async fn signed_out(State(auth): State<Auth>) -> Html<String> {
  let base = auth.base_path();

  Html(format!(
    "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width, initial-scale=1\"><meta name=color-scheme content=\"light dark\"><title>Signed out</title><p>You are signed out.</p><p><a href=\"{base}/auth/login\">Sign in again</a></p>"
  ))
}

/// Ends the session and removes its cookie. It changes something, so it is held to the same origin check as the API: no other site
/// may sign people out.
async fn logout(State(auth): State<Auth>, method: Method, headers: HeaderMap) -> Result<Response, AuthError> {
  auth.check_origin(&method, &headers)?;
  let cookie = auth.end_session(&headers).await?;

  Ok((StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response())
}

/// Only an address on this server may follow a login, or the login page would redirect anywhere. Without one, the editor.
fn local_path(next: Option<&str>, base: &str) -> String {
  match next {
    Some(path) if path.starts_with('/') && !path.starts_with("//") && !path.contains('\\') && !path.chars().any(char::is_control) => path.to_string(),
    _ => format!("{base}/"),
  }
}

fn escape(text: &str) -> String {
  text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn error_page(message: &str, base: &str) -> String {
  format!(
    "<!doctype html><meta charset=utf-8><title>Sign-in failed</title><p>{}</p><p><a href=\"{base}/auth/login\">Try again</a></p>",
    escape(message)
  )
}

#[cfg(test)]
mod tests {
  use axum::body::Body;
  use jiff::SignedDuration;
  use tower::ServiceExt;

  use super::*;
  use crate::{AccessRules, AuthSettings, Oidc, SessionSettings};

  #[tokio::test]
  async fn a_callback_is_accepted_only_from_the_browser_that_started_the_login() {
    let dir = std::env::temp_dir().join(format!("cairn-routes-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = cairn_store::connect_for_tests(&dir).await;
    let settings = AuthSettings {
      sessions: SessionSettings {
        idle: SignedDuration::from_hours(1),
        absolute: SignedDuration::from_hours(24),
        secure: true,
      },
      access: AccessRules {
        allow_any: true,
        ..Default::default()
      },
      public_origin: "https://docs.example.com".into(),
      base_path: String::new(),
    };
    let app = routes().with_state(Auth::oidc(Oidc::unreachable(), None, db.clone(), settings));

    let login = app.clone().oneshot(axum::http::Request::get("/auth/login").body(Body::empty()).unwrap()).await.unwrap();
    let set_cookie = login.headers()[header::SET_COOKIE].to_str().unwrap().to_string();
    let state = set_cookie.strip_prefix("__Host-login=").and_then(|rest| rest.split(';').next()).unwrap().to_string();
    assert!(set_cookie.contains("HttpOnly") && set_cookie.contains("Secure"), "{set_cookie}");

    // The attacker's callback address, opened in the victim's browser: no cookie, or the cookie of another login.
    for cookie in [None, Some("__Host-login=another")] {
      let mut request = axum::http::Request::get(format!("/auth/callback?code=c&state={state}"));
      if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
      }
      let response = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();

      assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{cookie:?}");
      assert!(response.headers().get_all(header::SET_COOKIE).iter().all(|c| !c.to_str().unwrap().starts_with("__Host-session")));
    }
    // Refusing it did not use the login up: its own browser can still finish it.
    assert!(take_login_state(&db, &state).await.unwrap().is_some());
  }

  #[test]
  fn without_a_next_address_the_login_returns_to_the_editor() {
    assert_eq!(local_path(None, "/_editor"), "/_editor/");
    assert_eq!(local_path(Some("//evil.example"), "/_editor"), "/_editor/");
  }

  #[test]
  fn only_local_paths_follow_a_login() {
    for (next, expected) in [
      (Some("/_editor/#page=1"), "/_editor/#page=1"),
      (Some("//evil.example"), "/"),
      (Some("https://evil.example"), "/"),
      (Some("/\\evil.example"), "/"),
      (Some("/a\nb"), "/"),
      (None, "/"),
    ] {
      assert_eq!(local_path(next, ""), expected, "{next:?}");
    }
  }
}
