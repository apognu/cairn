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
  /// `postgres://…` to use Postgres; without it the database is a SQLite file in the data directory.
  pub(crate) database_url: Option<String>,
  /// The address readers and editors use, without a trailing slash: the origin of redirects, cookies and canonical links.
  pub(crate) public_url: String,

  // Proxy
  pub(crate) trusted_proxies: crate::proxy::TrustedProxies,

  // Publishing
  pub(crate) target: TargetChoice,
  /// Where readers find the site: `PUBLIC_URL` when this server serves it, otherwise wherever the target puts it (`SITE_URL`).
  pub(crate) site_url: String,

  // OIDC
  pub(crate) oidc: OidcSettings,

  // GitHub (optional second way to sign in)
  pub(crate) github: Option<GithubSettings>,
}

#[derive(Debug, Clone)]
pub(crate) struct GithubSettings {
  pub(crate) client_id: String,
  pub(crate) client_secret: String,
  /// Organizations (`acme`) and teams (`acme/docs`) whose members may sign in, and email domains.
  pub(crate) access: yourdocs_auth::AccessRules,
  /// Overridable for tests and GitHub Enterprise.
  pub(crate) web_url: String,
  pub(crate) api_url: String,
}

/// Where releases go, chosen with `PUBLISH_TARGET` and read once at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetChoice {
  /// Written under the data directory and served by this process at `PUBLIC_URL`.
  SelfHosted,
  /// Written to a folder that something else serves.
  Folder(PathBuf),
  /// Uploaded to a Cloudflare Worker with static assets.
  Cloudflare(yourdocs_publish::CloudflareConfig),
  /// Uploaded to an S3 bucket (or compatible storage) that serves it as a website.
  S3(yourdocs_publish::S3Config),
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

fn validate_database_url(url: &str) -> anyhow::Result<String> {
  if url.starts_with("postgres://") || url.starts_with("postgresql://") {
    return Ok(url.to_string());
  }

  bail!("DATABASE_URL must be a postgres:// address; without it the database is a SQLite file in DATA_DIR")
}

/// `https://docs.example.com` for `https://docs.example.com/any/path?x`: what a browser sends as `Origin`.
pub(crate) fn origin_of(url: &str) -> String {
  let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));

  format!("{scheme}://{}", rest.split(['/', '?', '#']).next().unwrap_or_default())
}

/// The allow-list read from `<prefix>_ALLOWED_GROUPS`, `<prefix>_ALLOWED_EMAIL_DOMAINS` and, where it makes sense, `<prefix>_ALLOW_ANY_USER`.
fn rules(var: &dyn Fn(&str) -> Option<String>, prefix: &str, any: bool) -> yourdocs_auth::AccessRules {
  let list = |name: &str| {
    var(&format!("{prefix}_{name}"))
      .map(|v| v.split([',', ' ']).filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>())
      .unwrap_or_default()
  };

  yourdocs_auth::AccessRules {
    groups: list("ALLOWED_GROUPS"),
    email_domains: list("ALLOWED_EMAIL_DOMAINS").into_iter().map(|d| d.trim_start_matches('@').to_lowercase()).collect(),
    allow_any: any && var(&format!("{prefix}_ALLOW_ANY_USER")).is_some_and(|v| v.eq_ignore_ascii_case("true")),
  }
}

/// GitHub sign-in, when `GITHUB_CLIENT_ID` is set. Anyone can have a GitHub account, so who may sign in must be said.
fn github(var: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<Option<GithubSettings>> {
  let Some(client_id) = var("GITHUB_CLIENT_ID") else { return Ok(None) };
  let access = rules(var, "GITHUB", false);

  if access.is_empty() {
    bail!("GITHUB_CLIENT_ID is set: say who may sign in with GitHub with GITHUB_ALLOWED_GROUPS (organizations like acme, teams like acme/docs) or GITHUB_ALLOWED_EMAIL_DOMAINS");
  }

  Ok(Some(GithubSettings {
    client_id,
    client_secret: var("GITHUB_CLIENT_SECRET").context("GITHUB_CLIENT_SECRET is required with GITHUB_CLIENT_ID")?,
    access,
    web_url: var("GITHUB_AUTH_URL").unwrap_or_else(|| "https://github.com".into()).trim_end_matches('/').to_string(),
    api_url: var("GITHUB_API_URL").unwrap_or_else(|| "https://api.github.com".into()).trim_end_matches('/').to_string(),
  }))
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
      database_url: var("DATABASE_URL").map(|url| validate_database_url(&url)).transpose()?,
      data_dir: var("DATA_DIR").map_or_else(|| PathBuf::from("/data"), PathBuf::from),
      site_url: var("SITE_URL").map_or_else(|| public_url.clone(), |url| url.trim_end_matches('/').to_string()),
      public_url,
      trusted_proxies: crate::proxy::TrustedProxies::parse(&var("TRUSTED_PROXIES").unwrap_or_default()).map_err(|e| anyhow::anyhow!("TRUSTED_PROXIES: {e}"))?,
      target: match var("PUBLISH_TARGET").as_deref().unwrap_or("self-hosted") {
        "self-hosted" => TargetChoice::SelfHosted,
        "folder" => TargetChoice::Folder(
          var("PUBLISH_DIR")
            .map(PathBuf::from)
            .context("PUBLISH_TARGET=folder needs PUBLISH_DIR, the folder to write the site to")?,
        ),
        "cloudflare" => TargetChoice::Cloudflare(yourdocs_publish::CloudflareConfig {
          account_id: required("CLOUDFLARE_ACCOUNT_ID").context("PUBLISH_TARGET=cloudflare needs CLOUDFLARE_ACCOUNT_ID, CLOUDFLARE_API_TOKEN and CLOUDFLARE_WORKER")?,
          api_token: required("CLOUDFLARE_API_TOKEN")?,
          worker: required("CLOUDFLARE_WORKER")?,
          api_url: var("CLOUDFLARE_API_URL")
            .unwrap_or_else(|| "https://api.cloudflare.com/client/v4".into())
            .trim_end_matches('/')
            .to_string(),
        }),
        "s3" => {
          let endpoint = var("S3_ENDPOINT");
          TargetChoice::S3(yourdocs_publish::S3Config {
            bucket: required("S3_BUCKET").context("PUBLISH_TARGET=s3 needs S3_BUCKET, S3_ACCESS_KEY_ID and S3_SECRET_ACCESS_KEY")?,
            region: var("S3_REGION").unwrap_or_else(|| "us-east-1".into()),
            prefix: var("S3_PREFIX").unwrap_or_default(),
            access_key: required("S3_ACCESS_KEY_ID")?,
            secret_key: required("S3_SECRET_ACCESS_KEY")?,
            // A custom endpoint (MinIO, R2, …) is nearly always addressed by path; AWS by bucket name.
            path_style: var("S3_PATH_STYLE").map_or(endpoint.is_some(), |v| v.eq_ignore_ascii_case("true")),
            endpoint,
          })
        }
        other => bail!("PUBLISH_TARGET={other} is not available: use self-hosted, folder, cloudflare or s3"),
      },
      github: github(&var)?,
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
    assert_eq!(config.database_url, None);
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
  fn the_database_is_sqlite_unless_postgres_is_named() {
    let with = |url: &str| load(&[FULL, &[("DATABASE_URL", url)]].concat());

    assert_eq!(with("postgres://u:p@db/docs").unwrap().database_url.as_deref(), Some("postgres://u:p@db/docs"));
    assert!(with("mysql://db/docs").is_err());
  }

  #[test]
  fn the_origin_is_the_scheme_and_authority() {
    for (url, origin) in [("https://docs.example.com/a/b?x=1", "https://docs.example.com"), ("http://localhost:8080", "http://localhost:8080")] {
      assert_eq!(origin_of(url), origin);
    }
  }

  #[test]
  fn the_publish_target_is_chosen_with_its_own_settings() {
    let with = |extra: &[(&str, &str)]| load(&[FULL, extra].concat());

    assert_eq!(with(&[]).unwrap().target, TargetChoice::SelfHosted);
    assert_eq!(
      with(&[("PUBLISH_TARGET", "folder"), ("PUBLISH_DIR", "/srv/site")]).unwrap().target,
      TargetChoice::Folder("/srv/site".into())
    );
    assert!(with(&[("PUBLISH_TARGET", "folder")]).is_err());
    assert!(with(&[("PUBLISH_TARGET", "cloudflare"), ("CLOUDFLARE_ACCOUNT_ID", "a")]).is_err());
    let cloudflare = [
      ("PUBLISH_TARGET", "cloudflare"),
      ("CLOUDFLARE_ACCOUNT_ID", "a"),
      ("CLOUDFLARE_API_TOKEN", "t"),
      ("CLOUDFLARE_WORKER", "docs"),
      ("SITE_URL", "https://docs.example.org/"),
    ];
    let config = with(&cloudflare).unwrap();
    assert!(matches!(config.target, TargetChoice::Cloudflare(c) if c.worker == "docs"));
    assert_eq!(config.site_url, "https://docs.example.org", "the site is not served from the editor's address");
    assert_eq!(with(&[]).unwrap().site_url, "https://docs.example.com");

    assert!(with(&[("PUBLISH_TARGET", "s3"), ("S3_BUCKET", "b")]).is_err());
    let s3 = [
      ("PUBLISH_TARGET", "s3"),
      ("S3_BUCKET", "docs"),
      ("S3_ACCESS_KEY_ID", "k"),
      ("S3_SECRET_ACCESS_KEY", "s"),
      ("S3_ENDPOINT", "http://minio:9000"),
    ];
    let TargetChoice::S3(minio) = with(&s3).unwrap().target else { panic!("not S3") };
    assert!(minio.path_style, "a custom endpoint is addressed by path");
    let TargetChoice::S3(aws) = with(&s3[..4]).unwrap().target else { panic!("not S3") };
    assert!(!aws.path_style && aws.region == "us-east-1" && aws.endpoint.is_none());
    assert!(with(&[("PUBLISH_TARGET", "ftp")]).is_err());
  }

  #[test]
  fn github_needs_a_secret_and_an_allow_list() {
    let with = |extra: &[(&str, &str)]| load(&[FULL, extra].concat());
    let id = ("GITHUB_CLIENT_ID", "gh");

    assert!(with(&[]).unwrap().github.is_none());
    assert!(with(&[id, ("GITHUB_ALLOWED_GROUPS", "acme")]).is_err(), "no secret");
    assert!(with(&[id, ("GITHUB_CLIENT_SECRET", "s")]).is_err(), "nobody may sign in");
    // Anyone can have a GitHub account: there is no way to let everyone in.
    assert!(with(&[id, ("GITHUB_CLIENT_SECRET", "s"), ("GITHUB_ALLOW_ANY_USER", "true")]).is_err());

    let github = with(&[id, ("GITHUB_CLIENT_SECRET", "s"), ("GITHUB_ALLOWED_GROUPS", "acme, acme/docs")]).unwrap().github.unwrap();
    assert_eq!(github.access.groups, ["acme", "acme/docs"]);
    assert_eq!(github.web_url, "https://github.com");
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
