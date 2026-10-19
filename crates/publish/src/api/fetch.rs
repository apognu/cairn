use std::{
  net::{IpAddr, SocketAddr},
  sync::Arc,
  time::Duration,
};

use cairn_core::{ApiSource, GitRef};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use reqwest::{
  Client, StatusCode, Url,
  dns::{Addrs, Name, Resolve, Resolving},
  header, redirect,
};

use crate::PublishError;

/// A spec is never this big; the cap keeps a hostile or broken source from filling the memory of a publish.
const MAX_BYTES: usize = 20 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

// Keeps what a URL path holds as is, so refs and file paths stay readable and slashes keep separating segments.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');

/// Where GitHub's web and raw-file hosts are. Only tests and mirrors change them; any other host is refused unless it is HTTPS on a public address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiHosts {
  pub github: String,
  pub raw: String,
}

impl Default for ApiHosts {
  fn default() -> Self {
    ApiHosts {
      github: "https://github.com".into(),
      raw: "https://raw.githubusercontent.com".into(),
    }
  }
}

/// A spec file as it was served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
  pub bytes: Vec<u8>,
  /// The address the bytes came from, without redirects: relative `$ref`s resolve against it.
  pub url: String,
  /// The release tag the file was read at, for a source on a release.
  pub tag: Option<String>,
  pub etag: Option<String>,
}

/// Downloads specs anonymously over HTTPS: URLs as they are, and files or release assets of public GitHub repositories, without the GitHub API.
pub struct SourceFetcher {
  client: Client,
  // Never follows a redirect: `releases/latest` answers with one, and its target is the whole answer.
  plain: Client,
  hosts: ApiHosts,
  mirrors: Arc<Mirrors>,
  max_bytes: usize,
}

/// The GitHub hosts configured in place of the real ones (a mirror, or a test stand-in), which may be plain HTTP on the private network.
/// Only these exact origins get that leeway: a source typed by an author is still held to HTTPS on a public address.
#[derive(Debug, Default)]
struct Mirrors {
  origins: Vec<String>,
  /// Their host names, which may resolve to private addresses.
  names: Vec<String>,
}

impl Mirrors {
  fn of(hosts: &ApiHosts) -> Result<Self, PublishError> {
    if *hosts == ApiHosts::default() {
      return Ok(Mirrors::default());
    }

    let mut mirrors = Mirrors::default();
    for host in [&hosts.github, &hosts.raw] {
      let url = Url::parse(host).map_err(|_| PublishError::Other(anyhow::anyhow!("{host} is not a valid address for a GitHub mirror")))?;
      if !matches!(url.scheme(), "http" | "https") {
        return Err(PublishError::Other(anyhow::anyhow!("{host} is not an HTTP address for a GitHub mirror")));
      }

      mirrors.origins.push(url.origin().ascii_serialization());
      mirrors.names.extend(url.host_str().map(str::to_ascii_lowercase));
    }

    Ok(mirrors)
  }
}

impl SourceFetcher {
  pub fn new(hosts: ApiHosts) -> Result<Self, PublishError> {
    let mirrors = Arc::new(Mirrors::of(&hosts)?);
    let build = |policy: redirect::Policy| {
      Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("cairn/", env!("CARGO_PKG_VERSION")))
        .redirect(policy)
        .dns_resolver(Arc::new(PublicOnly { mirrors: mirrors.clone() }))
        .build()
        .map_err(|e| PublishError::Other(e.into()))
    };
    let follow = {
      let mirrors = mirrors.clone();
      redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
          attempt.error("too many redirects")
        } else if !allowed(attempt.url(), &mirrors) {
          attempt.error("redirected to a place sources may not use")
        } else {
          attempt.follow()
        }
      })
    };

    Ok(SourceFetcher {
      client: build(follow)?,
      plain: build(redirect::Policy::none())?,
      hosts,
      mirrors,
      max_bytes: MAX_BYTES,
    })
  }

  /// Fetches a configured source. A source on the latest release first looks up which tag that is.
  pub async fn fetch_source(&self, source: &ApiSource) -> Result<Fetched, PublishError> {
    let label = source.label();
    let fail = |message: String| PublishError::ApiSource { label: label.clone(), message };

    match source {
      ApiSource::Url { url, .. } => self.get(url).await.map_err(fail),
      ApiSource::Github { repo, path, git_ref, asset, .. } => {
        let tag = match git_ref {
          GitRef::LatestRelease => Some(self.latest_tag(repo).await.map_err(fail)?),
          GitRef::Tag { name } => Some(name.clone()),
          GitRef::Branch { .. } => None,
        };
        let reference = match (&tag, git_ref) {
          (Some(tag), _) => tag.clone(),
          (None, GitRef::Branch { name }) => name.clone(),
          (None, _) => unreachable!("only branches have no tag"),
        };
        let url = match asset {
          Some(asset) => {
            let tag = tag.as_deref().ok_or_else(|| fail("a release asset needs a release".into()))?;
            format!("{}/{repo}/releases/download/{}/{}", self.hosts.github, encode(tag), encode(asset))
          }
          None => format!("{}/{repo}/{}/{}", self.hosts.raw, encode(&reference), encode(path)),
        };

        let mut fetched = self.get(&url).await.map_err(fail)?;
        fetched.tag = tag;

        Ok(fetched)
      }
    }
  }

  /// The tag of a repository's latest release, read from where `releases/latest` redirects to.
  pub async fn latest_tag(&self, repo: &str) -> Result<String, String> {
    let url = format!("{}/{repo}/releases/latest", self.hosts.github);
    let response = self.plain.get(&url).send().await.map_err(describe)?;

    if !response.status().is_redirection() {
      return Err(match response.status() {
        StatusCode::NOT_FOUND | StatusCode::OK => format!("{repo} has no release"),
        status => format!("HTTP {status} for {url}"),
      });
    }

    let location = response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()).ok_or("the release lookup gave no address")?;
    let target = Url::parse(&url)
      .and_then(|base| base.join(location))
      .map_err(|_| "the release lookup gave an invalid address".to_string())?;
    let tag = target.path().split_once("/releases/tag/").map(|(_, tag)| tag).filter(|tag| !tag.is_empty());

    tag.map(|tag| percent_decode_str(tag).decode_utf8_lossy().into_owned()).ok_or_else(|| format!("{repo} has no release"))
  }

  /// Downloads one file. Other files a spec points to (`$ref`) come through here too.
  pub async fn get(&self, url: &str) -> Result<Fetched, String> {
    let parsed = Url::parse(url).map_err(|_| format!("{url} is not a valid address"))?;
    if !allowed(&parsed, &self.mirrors) {
      return Err(format!("{url} is not an HTTPS address on the public internet"));
    }

    let mut response = self.client.get(parsed).send().await.map_err(describe)?;
    if !response.status().is_success() {
      return Err(format!("HTTP {} for {url}", response.status()));
    }

    let etag = response.headers().get(header::ETAG).and_then(|v| v.to_str().ok()).map(str::to_string);
    if response.content_length().is_some_and(|n| n > self.max_bytes as u64) {
      return Err(format!("{url} is larger than {} MB", self.max_bytes / 1024 / 1024));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
      bytes.extend_from_slice(&chunk);
      if bytes.len() > self.max_bytes {
        return Err(format!("{url} is larger than {} MB", self.max_bytes / 1024 / 1024));
      }
    }

    Ok(Fetched {
      bytes,
      url: url.to_string(),
      tag: None,
      etag,
    })
  }
}

fn encode(path: &str) -> String {
  path.split('/').map(|segment| utf8_percent_encode(segment, SEGMENT).to_string()).collect::<Vec<_>>().join("/")
}

fn describe(error: reqwest::Error) -> String {
  if error.is_timeout() {
    "the request timed out".to_string()
  } else if error.is_redirect() {
    // The policy's own reason is in the source chain.
    std::error::Error::source(&error).map_or_else(|| "too many redirects".to_string(), ToString::to_string)
  } else if error.is_connect() {
    format!("could not connect: {}", error.without_url())
  } else {
    error.without_url().to_string()
  }
}

/// A configured mirror, or HTTPS that is not an IP address typed into the URL pointing inside the network. Names are checked when they resolve.
fn allowed(url: &Url, mirrors: &Mirrors) -> bool {
  if mirrors.origins.contains(&url.origin().ascii_serialization()) {
    return true;
  }

  let Some(host) = url.host_str() else { return false };
  let literal = host.trim_start_matches('[').trim_end_matches(']').parse::<IpAddr>();

  url.scheme() == "https" && literal.map_or(true, public)
}

/// Anything on this machine or the private network is off limits: a source is typed by an author, and the server may sit next to services
/// that trust it.
fn public(ip: IpAddr) -> bool {
  match ip {
    IpAddr::V4(ip) => {
      let [a, b, c, _] = ip.octets();
      !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || a == 0
        || a >= 240
        || (a == 100 && (64..128).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (18..20).contains(&b)))
    }
    IpAddr::V6(ip) => {
      let s = ip.segments();
      let embedded = |high: u16, low: u16| public(IpAddr::V4(std::net::Ipv4Addr::from((u32::from(high) << 16) | u32::from(low))));

      if let Some(mapped) = ip.to_ipv4_mapped() {
        return public(IpAddr::V4(mapped));
      }
      // These carry an IPv4 address that the network may route to: IPv4-compatible (`::a.b.c.d`, which also covers `::` and `::1`),
      // NAT64 (`64:ff9b::/96`) and 6to4 (`2002::/16`).
      if s[..6] == [0; 6] || s[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
        return embedded(s[6], s[7]);
      }
      if s[0] == 0x2002 {
        return embedded(s[1], s[2]);
      }

      !(ip.is_multicast()
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] & 0xffc0) == 0xfe80
        // Deprecated site-local, local-use NAT64 and Teredo, which hide where they lead.
        || (s[0] & 0xffc0) == 0xfec0
        || (s[0] == 0x64 && s[1] == 0xff9b && s[2] == 1)
        || (s[0] == 0x2001 && s[1] == 0))
    }
  }
}

/// Resolves names like the system does, then drops the private addresses, so a name pointing inside the network fails to connect. A
/// configured mirror may resolve anywhere.
struct PublicOnly {
  mirrors: Arc<Mirrors>,
}

impl Resolve for PublicOnly {
  fn resolve(&self, name: Name) -> Resolving {
    let mirror = self.mirrors.names.iter().any(|m| m.eq_ignore_ascii_case(name.as_str()));

    Box::pin(async move {
      let found: Vec<SocketAddr> = tokio::net::lookup_host((name.as_str(), 0)).await?.filter(|a| mirror || public(a.ip())).collect();
      if found.is_empty() {
        return Err(format!("{} does not resolve to a public address", name.as_str()).into());
      }

      Ok(Box::new(found.into_iter()) as Addrs)
    })
  }
}

#[cfg(test)]
mod tests {
  use axum::{
    Router,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::get,
  };

  use super::*;

  /// A stand-in for GitHub: the web host and the raw host are one local server.
  async fn stand_in() -> SourceFetcher {
    let app = Router::new()
      .route(
        "/acme/orders/releases/latest",
        get(|| async { (StatusCode::FOUND, [(header::LOCATION, "/acme/orders/releases/tag/v1.2.0")]) }),
      )
      .route(
        "/acme/slashes/releases/latest",
        get(|| async { (StatusCode::FOUND, [(header::LOCATION, "/acme/slashes/releases/tag/rel%2Fv2")]) }),
      )
      .route(
        "/acme/orders/releases/download/v1.2.0/openapi.json",
        get(|| async { ([(header::ETAG, "\"abc\"")], "{\"asset\":true}") }),
      )
      .route("/acme/orders/v1.2.0/api/spec.yaml", get(|| async { "at-tag" }))
      .route("/acme/orders/main/api/spec.yaml", get(|| async { "on-branch" }))
      .route("/acme/slashes/rel/v2/spec.yaml", get(|| async { "slashed" }))
      .route("/loop", get(|| async { (StatusCode::FOUND, [(header::LOCATION, "/loop")]) }))
      .route("/big", get(|| async { vec![b'x'; 4096] }))
      .route(
        "/echo-agent",
        get(|headers: HeaderMap| async move { headers.get(header::USER_AGENT).map(|v| v.as_bytes().to_vec()).unwrap_or_default().into_response() }),
      );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    SourceFetcher::new(ApiHosts { github: base.clone(), raw: base }).unwrap()
  }

  fn github(repo: &str, path: &str, git_ref: GitRef, asset: Option<&str>) -> ApiSource {
    ApiSource::Github {
      repo: repo.into(),
      path: path.into(),
      git_ref,
      asset: asset.map(Into::into),
      prefix: String::new(),
    }
  }

  fn text(fetched: &Fetched) -> &str {
    std::str::from_utf8(&fetched.bytes).unwrap()
  }

  #[tokio::test]
  async fn finds_the_latest_tag_from_the_redirect() {
    let fetcher = stand_in().await;

    assert_eq!(fetcher.latest_tag("acme/orders").await.unwrap(), "v1.2.0");
    assert_eq!(fetcher.latest_tag("acme/slashes").await.unwrap(), "rel/v2");
    assert_eq!(fetcher.latest_tag("acme/none").await.unwrap_err(), "acme/none has no release");
  }

  #[tokio::test]
  async fn reads_files_and_assets_at_the_tag_or_branch() {
    let fetcher = stand_in().await;

    let file = fetcher.fetch_source(&github("acme/orders", "api/spec.yaml", GitRef::LatestRelease, None)).await.unwrap();
    assert_eq!((text(&file), file.tag.as_deref()), ("at-tag", Some("v1.2.0")));
    assert!(file.url.ends_with("/acme/orders/v1.2.0/api/spec.yaml"));

    let pinned = fetcher
      .fetch_source(&github("acme/orders", "api/spec.yaml", GitRef::Tag { name: "v1.2.0".into() }, None))
      .await
      .unwrap();
    assert_eq!(text(&pinned), "at-tag");

    let branch = fetcher
      .fetch_source(&github("acme/orders", "api/spec.yaml", GitRef::Branch { name: "main".into() }, None))
      .await
      .unwrap();
    assert_eq!((text(&branch), branch.tag.as_deref()), ("on-branch", None));

    let asset = fetcher.fetch_source(&github("acme/orders", "", GitRef::LatestRelease, Some("openapi.json"))).await.unwrap();
    assert_eq!((text(&asset), asset.etag.as_deref(), asset.tag.as_deref()), ("{\"asset\":true}", Some("\"abc\""), Some("v1.2.0")));

    // A tag with a slash in it is a path, not an extra segment of the repository.
    let slashed = fetcher.fetch_source(&github("acme/slashes", "spec.yaml", GitRef::LatestRelease, None)).await.unwrap();
    assert_eq!(text(&slashed), "slashed");
  }

  #[tokio::test]
  async fn failures_name_the_source() {
    let fetcher = stand_in().await;

    let missing = github("acme/orders", "nope.yaml", GitRef::Tag { name: "v1.2.0".into() }, None);
    let message = fetcher.fetch_source(&missing).await.unwrap_err().to_string();
    assert!(message.starts_with("API source acme/orders nope.yaml: HTTP 404"), "{message}");

    let unreleased = github("acme/none", "a.yaml", GitRef::LatestRelease, None);
    let message = fetcher.fetch_source(&unreleased).await.unwrap_err().to_string();
    assert_eq!(message, "API source acme/none a.yaml: acme/none has no release");

    let looping = ApiSource::Url {
      url: format!("{}/loop", fetcher.hosts.github),
      prefix: String::new(),
    };
    let message = fetcher.fetch_source(&looping).await.unwrap_err().to_string();
    assert!(message.starts_with("API source") && message.contains("redirects"), "{message}");
  }

  #[tokio::test]
  async fn refuses_files_over_the_size_cap() {
    let mut fetcher = stand_in().await;
    fetcher.max_bytes = 1024;

    let error = fetcher.get(&format!("{}/big", fetcher.hosts.github)).await.unwrap_err();
    assert!(error.contains("is larger than"), "{error}");
  }

  #[tokio::test]
  async fn identifies_itself() {
    let fetcher = stand_in().await;
    let agent = fetcher.get(&format!("{}/echo-agent", fetcher.hosts.github)).await.unwrap();

    assert!(text(&agent).starts_with("cairn/"));
  }

  #[test]
  fn only_public_https_addresses_are_allowed() {
    let cases = [
      ("https://example.com/a.yaml", true),
      ("https://8.8.8.8/a.yaml", true),
      ("http://example.com/a.yaml", false),
      ("https://127.0.0.1/a.yaml", false),
      ("https://10.0.0.5/a.yaml", false),
      ("https://192.168.1.1/a.yaml", false),
      ("https://169.254.169.254/latest/meta-data", false),
      ("https://[::1]/a.yaml", false),
      ("https://[::ffff:10.0.0.1]/a.yaml", false),
      ("https://[fd00::1]/a.yaml", false),
      ("https://198.18.0.1/a.yaml", false),
      ("https://192.0.0.1/a.yaml", false),
      ("https://240.0.0.1/a.yaml", false),
      // IPv6 addresses that carry a private IPv4 one, and the same forms around a public one.
      ("https://[::10.0.0.1]/a.yaml", false),
      ("https://[64:ff9b::a9fe:a9fe]/a.yaml", false),
      ("https://[2002:a00:1::]/a.yaml", false),
      ("https://[64:ff9b::808:808]/a.yaml", true),
      ("https://[2002:808:808::]/a.yaml", true),
      ("https://[64:ff9b:1::1]/a.yaml", false),
      ("https://[2001:0:4136:e378::1]/a.yaml", false),
      ("https://[fec0::1]/a.yaml", false),
      ("https://[2606:4700::1111]/a.yaml", true),
      ("ftp://example.com/a.yaml", false),
    ];

    for (url, ok) in cases {
      assert_eq!(allowed(&Url::parse(url).unwrap(), &Mirrors::default()), ok, "{url}");
    }
  }

  #[tokio::test]
  async fn a_mirror_opens_only_its_own_origin() {
    let fetcher = stand_in().await;
    let mirror = Url::parse(&fetcher.hosts.github).unwrap();
    let port = mirror.port().unwrap();

    // (URL, allowed): the mirror itself, then what an author could type to reach the network around it.
    for (url, ok) in [
      (format!("{}/a.yaml", fetcher.hosts.github), true),
      (format!("http://127.0.0.1:{}/a.yaml", port + 1), false),
      (format!("https://127.0.0.1:{port}/a.yaml"), false),
      ("http://169.254.169.254/latest/meta-data".to_string(), false),
      ("http://10.0.0.5/a.yaml".to_string(), false),
    ] {
      assert_eq!(allowed(&Url::parse(&url).unwrap(), &fetcher.mirrors), ok, "{url}");
    }

    let error = fetcher.get("http://169.254.169.254/latest/meta-data").await.unwrap_err();
    assert!(error.contains("not an HTTPS address"), "{error}");
  }

  #[tokio::test]
  async fn a_default_fetcher_refuses_this_machine() {
    let fetcher = SourceFetcher::new(ApiHosts::default()).unwrap();

    let error = fetcher.get("http://127.0.0.1:1/a.yaml").await.unwrap_err();
    assert!(error.contains("not an HTTPS address"), "{error}");

    // A name that resolves to loopback is refused when it resolves, not only when typed as a number.
    let error = fetcher.get("https://localhost/a.yaml").await.unwrap_err();
    assert!(error.contains("public address") || error.contains("connect"), "{error}");
  }
}
