use serde::{Deserialize, Serialize};

const MAX_SOURCES: usize = 20;

/// What an API reference page is built from: public OpenAPI specs, merged into one document at every publish.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct ApiConfig {
  /// Merged in this order; the first one provides the document's `info` unless overridden.
  pub sources: Vec<ApiSource>,
  pub merge: MergeRules,
  pub display: ApiDisplay,
}

/// Where one spec comes from. Everything is fetched anonymously over HTTPS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiSource {
  /// A spec at a plain URL.
  Url {
    url: String,
    #[serde(default)]
    prefix: String,
  },
  /// A file of a public GitHub repository, or an asset of one of its releases when `asset` is set.
  Github {
    /// `owner/name`.
    repo: String,
    /// Path of the spec inside the repository; ignored for release assets.
    #[serde(default)]
    path: String,
    #[serde(default)]
    git_ref: GitRef,
    /// File name of a release asset, instead of a file in the repository.
    #[serde(default)]
    asset: Option<String>,
    #[serde(default)]
    prefix: String,
  },
}

/// Which version of a GitHub repository to read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GitRef {
  /// The tag of the repository's latest release, looked up at every build.
  #[default]
  LatestRelease,
  Tag {
    name: String,
  },
  Branch {
    name: String,
  },
}

/// How the specs combine. Conflicts are build errors; a source's prefix is how they get resolved.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct MergeRules {
  /// Replaces the `info` of the first source when set.
  pub title: Option<String>,
  pub version: Option<String>,
  pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct ApiDisplay {
  /// Server URLs shown in the reference; the specs' own are used when empty.
  pub servers: Vec<String>,
  pub hide_models: bool,
  /// Lets readers try requests from the page.
  pub request_client: bool,
  /// Shows a "Beta" pill after the page's name in the site's sidebar.
  pub beta: bool,
}

impl Default for ApiDisplay {
  fn default() -> Self {
    ApiDisplay {
      servers: Vec::new(),
      hide_models: false,
      request_client: true,
      beta: false,
    }
  }
}

/// An operation of a merged spec, as the page's fallback, the search index and the pre-publish checks read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Operation {
  /// Upper case: `GET`.
  pub method: String,
  pub path: String,
  pub summary: Option<String>,
  pub description: Option<String>,
  pub tags: Vec<String>,
  pub operation_id: Option<String>,
}

/// Where one spec was read from at a build, as recorded in the release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ResolvedSource {
  /// The source as written: a URL, or `owner/name` and the file or asset.
  pub label: String,
  /// The address the file came from.
  pub url: String,
  /// The release tag a GitHub source was read at.
  pub tag: Option<String>,
  pub etag: Option<String>,
}

impl ApiConfig {
  /// Checks what a user can type. Nothing is fetched here.
  pub fn validate(&self) -> Result<(), String> {
    if self.sources.len() > MAX_SOURCES {
      return Err(format!("an API reference takes at most {MAX_SOURCES} sources"));
    }

    for (index, source) in self.sources.iter().enumerate() {
      source.validate().map_err(|e| format!("source {}: {e}", index + 1))?;
    }

    for server in &self.display.servers {
      if !is_https_url(server) {
        return Err(format!("the server {server} must be an absolute http(s) URL"));
      }
    }

    for text in [&self.merge.title, &self.merge.version, &self.merge.description].into_iter().flatten() {
      if text.chars().count() > 2000 {
        return Err("the info overrides are limited to 2000 characters".into());
      }
    }

    Ok(())
  }
}

impl ApiSource {
  pub fn prefix(&self) -> &str {
    match self {
      ApiSource::Url { prefix, .. } | ApiSource::Github { prefix, .. } => prefix,
    }
  }

  /// A short name for messages: the URL, or `owner/name` with the file or asset.
  pub fn label(&self) -> String {
    match self {
      ApiSource::Url { url, .. } => url.clone(),
      ApiSource::Github { repo, path, asset, .. } => format!("{repo} {}", asset.as_deref().unwrap_or(path)),
    }
  }

  fn validate(&self) -> Result<(), String> {
    let prefix = self.prefix();
    if prefix.len() > 40 || !prefix.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')) {
      return Err("the prefix may only hold letters, digits, dashes and underscores".into());
    }

    match self {
      ApiSource::Url { url, .. } => {
        if !is_https_url(url) {
          return Err("the URL must be absolute and start with https://".into());
        }
      }
      ApiSource::Github { repo, path, git_ref, asset, .. } => {
        let mut parts = repo.split('/');
        let (owner, name) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
        let name_ok = |s: &str| !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
        if !name_ok(owner) || !name_ok(name) || parts.next().is_some() {
          return Err("the repository must look like owner/name".into());
        }

        match asset {
          Some(asset) if !path_ok(asset) || asset.contains('/') => return Err("the asset name is not valid".into()),
          None if !path_ok(path) => return Err("the path must be a file of the repository, without .. or a leading slash".into()),
          _ => {}
        }

        if let GitRef::Tag { name } | GitRef::Branch { name } = git_ref
          && (name.is_empty() || name.len() > 200 || name.contains(char::is_whitespace) || name.split('/').any(|s| s == ".." || s.is_empty()))
        {
          return Err("the tag or branch name is not valid".into());
        }

        if asset.is_some() && matches!(git_ref, GitRef::Branch { .. }) {
          return Err("a release asset needs a tag or the latest release, not a branch".into());
        }
      }
    }

    Ok(())
  }
}

fn path_ok(path: &str) -> bool {
  !path.is_empty() && path.len() <= 300 && !path.starts_with('/') && !path.contains(char::is_whitespace) && path.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}

fn is_https_url(url: &str) -> bool {
  url.len() <= 2048 && !url.contains(char::is_whitespace) && url.strip_prefix("https://").is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn github(repo: &str, path: &str, asset: Option<&str>) -> ApiSource {
    ApiSource::Github {
      repo: repo.into(),
      path: path.into(),
      git_ref: GitRef::LatestRelease,
      asset: asset.map(Into::into),
      prefix: String::new(),
    }
  }

  #[test]
  fn validates_sources() {
    let url = |url: &str| ApiSource::Url {
      url: url.into(),
      prefix: String::new(),
    };
    let cases = [
      (url("https://example.com/openapi.yaml"), true),
      (url("http://example.com/openapi.yaml"), false),
      (url("https:///x"), false),
      (github("acme/orders", "api/openapi.yaml", None), true),
      (github("acme/orders", "../secret", None), false),
      (github("acme/orders", "/abs", None), false),
      (github("acme", "a.yaml", None), false),
      (github("acme/orders/extra", "a.yaml", None), false),
      (github("acme/orders", "", Some("openapi.json")), true),
      (github("acme/orders", "", Some("a/b.json")), false),
      (github("acme/orders", "", None), false),
    ];

    for (source, ok) in cases {
      let config = ApiConfig {
        sources: vec![source.clone()],
        ..Default::default()
      };
      assert_eq!(config.validate().is_ok(), ok, "{source:?}");
    }
  }

  #[test]
  fn refuses_odd_prefixes_and_branch_assets() {
    let prefixed = ApiSource::Url {
      url: "https://example.com/a.yaml".into(),
      prefix: "bad prefix".into(),
    };
    assert!(
      ApiConfig {
        sources: vec![prefixed],
        ..Default::default()
      }
      .validate()
      .is_err()
    );

    let branch_asset = ApiSource::Github {
      repo: "acme/orders".into(),
      path: String::new(),
      git_ref: GitRef::Branch { name: "main".into() },
      asset: Some("a.json".into()),
      prefix: String::new(),
    };
    assert!(
      ApiConfig {
        sources: vec![branch_asset],
        ..Default::default()
      }
      .validate()
      .is_err()
    );
  }
}
