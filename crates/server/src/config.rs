//! The deployment's settings, read from the environment by `serve`.

use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use anyhow::{Context, bail};
use jiff::{SignedDuration, Span, SpanRelativeTo};

/// Everything `serve` is told through environment variables.
#[derive(Debug, Clone)]
pub(crate) struct Config {
  // Server
  pub(crate) bind: SocketAddr,
  pub(crate) data_dir: PathBuf,
  /// The address readers and editors use, without a trailing slash: the origin of redirects, cookies and canonical links.
  pub(crate) public_url: String,

  // OIDC
  pub(crate) oidc: OidcSettings,
}

#[derive(Debug, Clone)]
pub(crate) struct OidcSettings {
  pub(crate) issuer: String,
  pub(crate) client_id: String,
  pub(crate) client_secret: String,
  pub(crate) scopes: Vec<String>,
  pub(crate) groups_claim: String,
  /// A session unused for this long ends.
  pub(crate) session_idle: SignedDuration,
  /// A session ends this long after the login.
  pub(crate) session_absolute: SignedDuration,
  pub(crate) access: yourdocs_auth::AccessRules,
}

impl OidcSettings {
  pub(crate) fn sessions(&self, secure: bool) -> yourdocs_auth::SessionSettings {
    yourdocs_auth::SessionSettings {
      idle: self.session_idle,
      absolute: self.session_absolute,
      secure,
    }
  }
}

impl From<OidcSettings> for yourdocs_auth::OidcConfig {
  fn from(settings: OidcSettings) -> Self {
    Self {
      issuer: settings.issuer,
      client_id: settings.client_id,
      client_secret: settings.client_secret,
      scopes: settings.scopes,
      groups_claim: settings.groups_claim,
    }
  }
}

/// `https://docs.example.com` for `https://docs.example.com/any/path?x`: what a browser sends as `Origin`.
pub(crate) fn origin_of(url: &str) -> String {
  let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));

  format!("{scheme}://{}", rest.split(['/', '?', '#']).next().unwrap_or_default())
}

/// The sign-in allow-list. One of the three must be set, so that opening the editor to a whole provider is a decision.
fn access(var: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<yourdocs_auth::AccessRules> {
  let list = |name: &str| {
    var(name)
      .map(|v| v.split([',', ' ']).filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>())
      .unwrap_or_default()
  };
  let rules = yourdocs_auth::AccessRules {
    groups: list("OIDC_ALLOWED_GROUPS"),
    email_domains: list("OIDC_ALLOWED_EMAIL_DOMAINS").into_iter().map(|d| d.trim_start_matches('@').to_lowercase()).collect(),
    allow_any: var("OIDC_ALLOW_ANY_USER").is_some_and(|v| v.eq_ignore_ascii_case("true")),
  };

  if rules.is_empty() {
    bail!("say who may sign in: set OIDC_ALLOWED_GROUPS or OIDC_ALLOWED_EMAIL_DOMAINS, or OIDC_ALLOW_ANY_USER=true to accept everyone the provider authenticates");
  }

  Ok(rules)
}

impl Config {
  pub(crate) fn from_env() -> anyhow::Result<Self> {
    Self::from_lookup(|name| std::env::var(name).ok())
  }

  fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
    let var = |name: &str| get(name).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let required = |name: &str| var(name).with_context(|| format!("{name} is required"));
    let parsed =
      |name: &str, default: &str| -> anyhow::Result<_> { SocketAddr::from_str(&var(name).unwrap_or_else(|| default.into())).with_context(|| format!("{name} is not an address like 0.0.0.0:8080")) };

    let duration = |name: &str, default: &str| -> anyhow::Result<SignedDuration> {
      let span: Span = var(name).unwrap_or_else(|| default.into()).parse().with_context(|| format!("{name} is not a duration like 8h"))?;

      span.to_duration(SpanRelativeTo::days_are_24_hours()).with_context(|| format!("{name} is not a duration like 8h"))
    };

    let public_url = required("PUBLIC_URL")?.trim_end_matches('/').to_string();
    // The host without its port; an IPv6 address keeps its brackets, whose colons are not a port's.
    let host = public_url.split_once("://").map(|(_, rest)| {
      let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
      match authority.split_once(']') {
        Some((address, _)) if authority.starts_with('[') => format!("{address}]"),
        _ => authority.split(':').next().unwrap_or_default().to_string(),
      }
    });
    match (public_url.split_once("://").map(|(scheme, _)| scheme), host.as_deref()) {
      (Some("https"), _) => {}
      // Sessions travel in a cookie that is only safe over TLS; plain HTTP is for trying things out on this machine.
      (Some("http"), Some("localhost" | "127.0.0.1" | "[::1]")) => {}
      _ => bail!("PUBLIC_URL must be the https:// address people use (http:// only for localhost), got {public_url}"),
    }

    Ok(Self {
      bind: parsed("BIND", "0.0.0.0:8080")?,
      data_dir: var("DATA_DIR").map_or_else(|| PathBuf::from("/data"), PathBuf::from),
      public_url,
      oidc: OidcSettings {
        issuer: required("OIDC_ISSUER").context("sign-in is mandatory outside `dev`: set OIDC_ISSUER, OIDC_CLIENT_ID and OIDC_CLIENT_SECRET")?,
        client_id: required("OIDC_CLIENT_ID")?,
        client_secret: required("OIDC_CLIENT_SECRET")?,
        scopes: var("OIDC_SCOPES").map_or_else(|| vec!["groups".to_string()], |v| v.split([',', ' ']).filter(|s| !s.is_empty()).map(String::from).collect()),
        groups_claim: var("OIDC_GROUPS_CLAIM").unwrap_or_else(|| "groups".into()),
        access: access(&var)?,
        session_idle: duration("SESSION_IDLE", "8h")?,
        session_absolute: duration("SESSION_ABSOLUTE", "168h")?,
      },
    })
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  fn load(pairs: &[(&str, &str)]) -> anyhow::Result<Config> {
    let vars: HashMap<_, _> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    Config::from_lookup(|name| vars.get(name).cloned())
  }

  const FULL: &[(&str, &str)] = &[
    ("PUBLIC_URL", "https://docs.example.com/"),
    ("OIDC_ISSUER", "https://idp.example"),
    ("OIDC_CLIENT_ID", "yourdocs"),
    ("OIDC_CLIENT_SECRET", "secret"),
    ("OIDC_ALLOWED_EMAIL_DOMAINS", "@Example.com"),
  ];

  #[test]
  fn a_complete_environment_loads_with_defaults() {
    let config = load(FULL).unwrap();

    assert_eq!(config.public_url, "https://docs.example.com");
    assert_eq!(config.bind.port(), 8080);
    assert_eq!(config.oidc.groups_claim, "groups");
    assert_eq!(config.oidc.access.email_domains, ["example.com"]);
    assert_eq!((config.oidc.session_idle.as_secs(), config.oidc.session_absolute.as_secs()), (8 * 3600, 7 * 24 * 3600));
  }

  #[test]
  fn the_public_address_must_be_https_unless_it_is_this_machine() {
    for (url, ok) in [
      ("https://docs.example.com", true),
      ("http://localhost:8080", true),
      ("http://127.0.0.1:8080", true),
      ("http://[::1]:8080", true),
      ("http://localhost.evil.example", false),
      ("http://docs.example.com", false),
      ("docs.example.com", false),
      ("ftp://docs.example.com", false),
    ] {
      let pairs: Vec<_> = FULL.iter().map(|(k, v)| (*k, if *k == "PUBLIC_URL" { url } else { *v })).collect();

      assert_eq!(load(&pairs).is_ok(), ok, "{url}");
    }
  }

  #[test]
  fn the_origin_is_the_scheme_and_authority() {
    for (url, origin) in [("https://docs.example.com/a/b?x=1", "https://docs.example.com"), ("http://localhost:8080", "http://localhost:8080")] {
      assert_eq!(origin_of(url), origin);
    }
  }

  #[test]
  fn nobody_is_let_in_by_default() {
    let pairs: Vec<_> = FULL.iter().filter(|(k, _)| *k != "OIDC_ALLOWED_EMAIL_DOMAINS").copied().collect();

    assert!(format!("{:#}", load(&pairs).expect_err("must refuse")).contains("OIDC_ALLOW_ANY_USER"));
    assert!(load(&[&pairs[..], &[("OIDC_ALLOW_ANY_USER", "true")]].concat()).is_ok());
  }

  #[test]
  fn missing_settings_are_named() {
    for (without, expected) in [
      ("OIDC_ISSUER", "mandatory"),
      ("OIDC_CLIENT_ID", "OIDC_CLIENT_ID"),
      ("OIDC_CLIENT_SECRET", "OIDC_CLIENT_SECRET"),
      ("PUBLIC_URL", "PUBLIC_URL"),
    ] {
      let pairs: Vec<_> = FULL.iter().filter(|(k, _)| *k != without).copied().collect();
      let error = format!("{:#}", load(&pairs).expect_err("must refuse"));

      assert!(error.contains(expected), "{without}: {error}");
    }
  }
}
