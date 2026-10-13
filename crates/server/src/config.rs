//! The deployment's settings, read from the environment by `serve`.

use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use anyhow::{Context, bail};

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

impl Config {
  pub(crate) fn from_env() -> anyhow::Result<Self> {
    Self::from_lookup(|name| std::env::var(name).ok())
  }

  fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
    let var = |name: &str| get(name).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let required = |name: &str| var(name).with_context(|| format!("{name} is required"));
    let parsed =
      |name: &str, default: &str| -> anyhow::Result<_> { SocketAddr::from_str(&var(name).unwrap_or_else(|| default.into())).with_context(|| format!("{name} is not an address like 0.0.0.0:8080")) };

    let public_url = required("PUBLIC_URL")?.trim_end_matches('/').to_string();
    if !public_url.starts_with("http://") && !public_url.starts_with("https://") {
      bail!("PUBLIC_URL must start with https:// (or http:// on localhost), got {public_url}");
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
  ];

  #[test]
  fn a_complete_environment_loads_with_defaults() {
    let config = load(FULL).unwrap();

    assert_eq!(config.public_url, "https://docs.example.com");
    assert_eq!(config.bind.port(), 8080);
    assert_eq!(config.oidc.groups_claim, "groups");
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
