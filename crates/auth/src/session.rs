//! Session cookies: a random token in the browser, its hash in the database.

use axum::http::{HeaderMap, header};
use jiff::SignedDuration;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::AuthError;

/// How long a session lives, and whether the cookie can use the `__Host-` prefix.
#[derive(Debug, Clone, Copy)]
pub struct SessionSettings {
  /// A session unused for this long ends.
  pub idle: SignedDuration,
  /// A session ends this long after the login, however busy.
  pub absolute: SignedDuration,
  /// Behind HTTPS (the public address is `https://`): the cookie is `Secure` and named with the `__Host-` prefix, which makes browsers
  /// refuse it from subdomains and for other paths. Plain HTTP is for trying things out on localhost.
  pub secure: bool,
}

impl SessionSettings {
  pub(crate) fn cookie_name(&self) -> &'static str {
    if self.secure { "__Host-session" } else { "session" }
  }

  /// The `Set-Cookie` value that stores a session token.
  pub(crate) fn set_cookie(&self, token: &str) -> String {
    self.cookie(token, self.absolute.as_secs())
  }

  /// The `Set-Cookie` value that removes the cookie.
  pub(crate) fn clear_cookie(&self) -> String {
    self.cookie("", 0)
  }

  fn cookie(&self, value: &str, max_age: i64) -> String {
    let secure = if self.secure { "; Secure" } else { "" };

    format!("{}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}", self.cookie_name())
  }

  /// The session token in a request's cookies.
  pub(crate) fn token<'a>(&self, headers: &'a HeaderMap) -> Option<&'a str> {
    let name = self.cookie_name();

    headers
      .get_all(header::COOKIE)
      .iter()
      .filter_map(|value| value.to_str().ok())
      .flat_map(|value| value.split(';'))
      .filter_map(|pair| pair.trim().split_once('='))
      .find(|(key, value)| *key == name && !value.is_empty())
      .map(|(_, value)| value)
  }
}

/// A new session token: 256 random bits.
pub(crate) fn new_token() -> String {
  use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

  let mut bytes = [0u8; 32];
  rand::rng().fill_bytes(&mut bytes);

  URL_SAFE_NO_PAD.encode(bytes)
}

/// What is stored for a token. A fast hash is enough: the token is random, not a password.
pub(crate) fn hash_token(token: &str) -> String {
  Sha256::digest(token.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

impl From<jiff::Error> for AuthError {
  fn from(error: jiff::Error) -> Self {
    AuthError::Other(error.into())
  }
}

#[cfg(test)]
mod tests {
  use axum::http::HeaderValue;

  use super::*;

  fn settings(secure: bool) -> SessionSettings {
    SessionSettings {
      idle: SignedDuration::from_hours(8),
      absolute: SignedDuration::from_hours(24),
      secure,
    }
  }

  #[test]
  fn the_cookie_is_found_among_others_and_named_for_the_scheme() {
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, HeaderValue::from_static("theme=dark; __Host-session=abc123; other=1"));

    assert_eq!(settings(true).token(&headers), Some("abc123"));
    // A cookie of the other name is not ours: a plain-HTTP deployment does not accept the secure one.
    assert_eq!(settings(false).token(&headers), None);
  }

  #[test]
  fn the_cookie_is_hardened() {
    let cookie = settings(true).set_cookie("tok");

    for part in ["__Host-session=tok", "Path=/", "HttpOnly", "SameSite=Lax", "Secure", "Max-Age=86400"] {
      assert!(cookie.contains(part), "{cookie}");
    }
    assert!(settings(true).clear_cookie().contains("Max-Age=0"));
  }

  #[test]
  fn tokens_are_long_and_distinct_and_hashed_for_storage() {
    let (a, b) = (new_token(), new_token());

    assert_ne!(a, b);
    assert_eq!(a.len(), 43);
    assert_eq!(hash_token(&a).len(), 64);
    assert_ne!(hash_token(&a), a);
  }
}
