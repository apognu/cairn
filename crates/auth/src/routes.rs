use axum::{
  Router,
  extract::{Query, State},
  http::{HeaderMap, HeaderValue, StatusCode, header},
  response::{Html, IntoResponse, Redirect, Response},
  routing::{get, post},
};
use serde::Deserialize;
use yourdocs_store::{LoginState, save_login_state, take_login_state, upsert_user};

use crate::{Auth, AuthError, Mode};

pub(crate) fn routes() -> Router<Auth> {
  Router::new()
    .route("/auth/login", get(login))
    .route("/auth/callback", get(callback))
    .route("/auth/logout", post(logout))
}

#[derive(Deserialize)]
struct LoginQuery {
  next: Option<String>,
}

/// Starts the login: remembers the PKCE verifier and nonce under the state value, and sends the browser to the provider.
async fn login(State(auth): State<Auth>, Query(query): Query<LoginQuery>) -> Result<Redirect, AuthError> {
  let Mode::Oidc(oidc) = &*auth.0 else {
    return Err(AuthError::Login("this server has no login".into()));
  };
  let authorization = oidc.oidc.authorize();

  let login = LoginState {
    verifier: authorization.verifier,
    nonce: authorization.nonce,
    next: local_path(query.next.as_deref()),
  };
  save_login_state(&oidc.db, &authorization.state, &login).await?;

  Ok(Redirect::to(&authorization.url))
}

#[derive(Deserialize)]
struct CallbackQuery {
  code: Option<String>,
  state: Option<String>,
  error: Option<String>,
  error_description: Option<String>,
}

/// The provider sends the browser back here with a code. Failures are shown as a page, since a person is looking at it.
async fn callback(State(auth): State<Auth>, Query(query): Query<CallbackQuery>) -> Response {
  match finish_login(&auth, query).await {
    Ok((cookie, next)) => ([(header::SET_COOKIE, cookie)], Redirect::to(&next)).into_response(),
    Err(error) => {
      let message = match &error {
        AuthError::Login(message) => message.clone(),
        other => {
          tracing::error!(error = %other, "login failed");
          "The sign-in could not be completed. Try again.".to_string()
        }
      };
      (StatusCode::BAD_REQUEST, Html(error_page(&message))).into_response()
    }
  }
}

/// Completes a login; returns the cookie that starts the session and where to send the browser.
async fn finish_login(auth: &Auth, query: CallbackQuery) -> Result<(HeaderValue, String), AuthError> {
  let Mode::Oidc(oidc) = &*auth.0 else {
    return Err(AuthError::Login("this server has no login".into()));
  };

  if let Some(error) = query.error {
    return Err(AuthError::Login(format!("the provider refused the sign-in: {}", query.error_description.unwrap_or(error))));
  }
  let (Some(code), Some(state)) = (query.code, query.state) else {
    return Err(AuthError::Login("the sign-in answer is incomplete".into()));
  };
  let login = take_login_state(&oidc.db, &state)
    .await?
    .ok_or_else(|| AuthError::Login("this sign-in expired or was already used; start again".into()))?;

  let identity = oidc.oidc.complete(code, login.verifier, login.nonce).await?;
  // Checked before the user is stored: someone refused leaves no trace.
  if !oidc.access.allows(&identity) {
    tracing::warn!(issuer = %identity.issuer, subject = %identity.subject, "sign-in refused: not in an allowed group or domain");
    return Err(AuthError::Login(
      "Your account is not allowed to use this site. Ask an administrator to add you to an allowed group.".into(),
    ));
  }
  let user = upsert_user(&oidc.db, &identity.issuer, &identity.subject, &identity.email, &identity.name).await?;
  tracing::info!(user_id = %user.id, "signed in");

  let cookie = auth.start_session(user.id).await?;

  Ok((HeaderValue::from_str(&cookie).map_err(|e| AuthError::Other(e.into()))?, login.next))
}

/// Ends the session and removes its cookie.
async fn logout(State(auth): State<Auth>, headers: HeaderMap) -> Result<Response, AuthError> {
  let cookie = auth.end_session(&headers).await?;

  Ok((StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response())
}

/// Only an address on this server may follow a login, or the login page would redirect anywhere.
fn local_path(next: Option<&str>) -> String {
  match next {
    Some(path) if path.starts_with('/') && !path.starts_with("//") && !path.contains('\\') && !path.chars().any(char::is_control) => path.to_string(),
    _ => "/".to_string(),
  }
}

fn escape(text: &str) -> String {
  text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn error_page(message: &str) -> String {
  format!(
    "<!doctype html><meta charset=utf-8><title>Sign-in failed</title><p>{}</p><p><a href=\"/auth/login\">Try again</a></p>",
    escape(message)
  )
}

#[cfg(test)]
mod tests {
  use super::*;

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
      assert_eq!(local_path(next), expected, "{next:?}");
    }
  }
}
