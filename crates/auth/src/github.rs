//! Sign-in with GitHub, for teams whose people already have a GitHub account. GitHub is an OAuth provider, not an OpenID Connect one:
//! there is no ID token, so the login reads the user, their verified email and their organizations and teams from the API.

use openidconnect::{CsrfToken, PkceCodeChallenge, reqwest};
use serde::Deserialize;

use crate::{AccessRules, AuthError, Authorization, Identity};

/// The issuer recorded for GitHub users; their subject is the numeric GitHub user ID, which survives renames.
pub const GITHUB_ISSUER: &str = "https://github.com";

#[derive(Debug, Clone)]
pub struct GithubConfig {
  pub client_id: String,
  pub client_secret: String,
  /// Where the browser goes to authorize and where the code is traded (`https://github.com`); other values are for tests and GitHub Enterprise.
  pub web_url: String,
  /// The API (`https://api.github.com`).
  pub api_url: String,
  /// Who may sign in: organizations (`acme`) and teams (`acme/docs`) go in `groups`. Unlike the main provider, there is no way to let
  /// everyone in: anyone can have a GitHub account.
  pub access: AccessRules,
}

pub struct Github {
  config: GithubConfig,
  http: reqwest::Client,
  redirect: String,
}

#[derive(Deserialize)]
struct Token {
  access_token: Option<String>,
  error_description: Option<String>,
}

#[derive(Deserialize)]
struct UserInfo {
  id: u64,
  login: String,
  name: Option<String>,
}

#[derive(Deserialize)]
struct Email {
  email: String,
  primary: bool,
  verified: bool,
}

#[derive(Deserialize)]
struct Org {
  login: String,
}

#[derive(Deserialize)]
struct Team {
  slug: String,
  organization: Org,
}

/// Pages of up to 100 are read this many times: more organizations or teams than that are not an editing team.
const MAX_PAGES: u32 = 5;

impl Github {
  pub fn new(config: GithubConfig, redirect: &str) -> Result<Self, AuthError> {
    let http = reqwest::ClientBuilder::new()
      .redirect(reqwest::redirect::Policy::none())
      .timeout(std::time::Duration::from_secs(10))
      .user_agent("cairn")
      .build()
      .map_err(|e| AuthError::Other(e.into()))?;

    Ok(Self {
      config,
      http,
      redirect: redirect.to_string(),
    })
  }

  pub(crate) fn allows(&self, identity: &Identity) -> bool {
    self.config.access.allows(identity)
  }

  /// The address to send the browser to, with a fresh state and PKCE challenge. There is no nonce: GitHub issues no ID token.
  pub fn authorize(&self) -> Authorization {
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let state = CsrfToken::new_random().secret().clone();
    let query = [
      ("client_id", self.config.client_id.as_str()),
      ("redirect_uri", self.redirect.as_str()),
      // `read:org` shows the teams of organizations that restrict third-party access only when the organization approves the application.
      ("scope", "read:user user:email read:org"),
      ("state", state.as_str()),
      ("code_challenge", challenge.as_str()),
      ("code_challenge_method", "S256"),
    ];
    let query = query.iter().map(|(k, v)| format!("{k}={}", percent_encode(v))).collect::<Vec<_>>().join("&");

    Authorization {
      url: format!("{}/login/oauth/authorize?{query}", self.config.web_url),
      state,
      nonce: String::new(),
      verifier: verifier.secret().clone(),
    }
  }

  /// Trades the code for a token and reads who the user is. Their groups are `org` for each organization and `org/team` for each team.
  pub async fn complete(&self, code: &str, verifier: &str) -> Result<Identity, AuthError> {
    let token = self.token(code, verifier).await?;

    let user: UserInfo = self.get(&token, "/user").await?;
    let emails: Vec<Email> = self.get(&token, "/user/emails").await.unwrap_or_default();
    let orgs: Vec<Org> = self.pages(&token, "/user/orgs").await?;
    let teams: Vec<Team> = self.pages(&token, "/user/teams").await?;

    let primary = emails.into_iter().find(|e| e.primary && e.verified);
    let groups = orgs
      .into_iter()
      .map(|o| o.login)
      .chain(teams.into_iter().map(|t| format!("{}/{}", t.organization.login, t.slug)))
      .collect();

    Ok(Identity {
      issuer: GITHUB_ISSUER.into(),
      subject: user.id.to_string(),
      email_verified: primary.is_some(),
      email: primary.map(|e| e.email).unwrap_or_default(),
      name: user.name.filter(|n| !n.is_empty()).unwrap_or(user.login),
      groups,
    })
  }

  async fn token(&self, code: &str, verifier: &str) -> Result<String, AuthError> {
    let response = self
      .http
      .post(format!("{}/login/oauth/access_token", self.config.web_url))
      .header("accept", "application/json")
      .form(&[
        ("client_id", self.config.client_id.as_str()),
        ("client_secret", self.config.client_secret.as_str()),
        ("code", code),
        ("redirect_uri", self.redirect.as_str()),
        ("code_verifier", verifier),
      ])
      .send()
      .await
      .map_err(|e| AuthError::Login(format!("cannot reach GitHub: {e}")))?;
    let body = response.bytes().await.map_err(|e| AuthError::Login(format!("cannot read GitHub's answer: {e}")))?;
    let token: Token = serde_json::from_slice(&body).map_err(|e| AuthError::Login(format!("GitHub's answer is not understood: {e}")))?;

    token
      .access_token
      .ok_or_else(|| AuthError::Login(format!("GitHub refused the code: {}", token.error_description.unwrap_or_default())))
  }

  async fn get<T: for<'de> Deserialize<'de>>(&self, token: &str, path: &str) -> Result<T, AuthError> {
    let response = self
      .http
      .get(format!("{}{path}", self.config.api_url))
      .bearer_auth(token)
      .header("accept", "application/vnd.github+json")
      .send()
      .await
      .map_err(|e| AuthError::Login(format!("cannot reach GitHub: {e}")))?;

    if !response.status().is_success() {
      return Err(AuthError::Login(format!("GitHub answered {} for {path}", response.status())));
    }
    let body = response.bytes().await.map_err(|e| AuthError::Login(format!("cannot read GitHub's answer: {e}")))?;

    serde_json::from_slice(&body).map_err(|e| AuthError::Login(format!("GitHub's answer for {path} is not understood: {e}")))
  }

  async fn pages<T: for<'de> Deserialize<'de>>(&self, token: &str, path: &str) -> Result<Vec<T>, AuthError> {
    let mut all = Vec::new();

    for page in 1..=MAX_PAGES {
      let items: Vec<T> = self.get(token, &format!("{path}?per_page=100&page={page}")).await?;
      let last = items.len() < 100;
      all.extend(items);

      if last {
        break;
      }
    }

    Ok(all)
  }
}

fn percent_encode(text: &str) -> String {
  text.bytes().fold(String::new(), |mut out, byte| {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
      _ => out.push_str(&format!("%{byte:02X}")),
    }
    out
  })
}

#[cfg(test)]
mod tests {
  use axum::{
    Json, Router,
    extract::Form,
    http::HeaderMap,
    routing::{get, post},
  };
  use serde_json::json;

  use super::*;

  /// A GitHub that knows one user, `alice` (id 42), with one organization and two teams.
  async fn stand_in() -> String {
    let app = Router::new()
      .route(
        "/login/oauth/access_token",
        post(|Form(form): Form<std::collections::HashMap<String, String>>| async move {
          if form.get("code").map(String::as_str) == Some("good") && form.get("code_verifier").is_some_and(|v| !v.is_empty()) {
            Json(json!({ "access_token": "tok", "token_type": "bearer" }))
          } else {
            Json(json!({ "error": "bad_verification_code", "error_description": "The code passed is incorrect or expired." }))
          }
        }),
      )
      .route(
        "/user",
        get(|headers: HeaderMap| async move {
          assert_eq!(headers["authorization"], "Bearer tok");
          Json(json!({ "id": 42, "login": "alice", "name": null }))
        }),
      )
      .route(
        "/user/emails",
        get(|| async { Json(json!([{ "email": "old@example.com", "primary": false, "verified": true }, { "email": "alice@example.com", "primary": true, "verified": true }])) }),
      )
      .route("/user/orgs", get(|| async { Json(json!([{ "login": "acme" }])) }))
      .route(
        "/user/teams",
        get(|| async { Json(json!([{ "slug": "docs", "organization": { "login": "acme" } }, { "slug": "ops", "organization": { "login": "acme" } }])) }),
      );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    format!("http://{address}")
  }

  fn github(url: &str) -> Github {
    Github::new(
      GithubConfig {
        client_id: "id".into(),
        client_secret: "secret".into(),
        web_url: url.into(),
        api_url: url.into(),
        access: AccessRules::default(),
      },
      "https://docs.example.com/_editor/auth/github/callback",
    )
    .unwrap()
  }

  #[tokio::test]
  async fn the_user_comes_back_with_their_verified_email_and_teams() {
    let url = stand_in().await;
    let identity = github(&url).complete("good", "verifier").await.unwrap();

    assert_eq!((identity.issuer.as_str(), identity.subject.as_str()), (GITHUB_ISSUER, "42"));
    assert_eq!((identity.email.as_str(), identity.email_verified, identity.name.as_str()), ("alice@example.com", true, "alice"));
    assert_eq!(identity.groups, ["acme", "acme/docs", "acme/ops"]);
  }

  #[tokio::test]
  async fn a_refused_code_is_a_login_error() {
    let url = stand_in().await;

    assert!(matches!(github(&url).complete("stale", "verifier").await, Err(AuthError::Login(m)) if m.contains("incorrect or expired")));
  }

  #[test]
  fn the_authorization_address_carries_pkce_and_the_callback() {
    let authorization = github("https://github.com").authorize();

    for part in [
      "https://github.com/login/oauth/authorize?",
      "client_id=id",
      "code_challenge_method=S256",
      "redirect_uri=https%3A%2F%2Fdocs.example.com%2F_editor%2Fauth%2Fgithub%2Fcallback",
      &format!("state={}", authorization.state),
    ] {
      assert!(authorization.url.contains(part), "{part}: {}", authorization.url);
    }
    assert!(!authorization.verifier.is_empty());
  }
}
