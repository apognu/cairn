//! The S3 target (and S3-compatible storage: MinIO, R2, …): the site's files become objects, and a bucket's website endpoint serves them.
//!
//! There is no atomic switch on S3, so the order narrows the window in which a reader can see a mix: new assets go first, then the pages
//! that refer to them, and the files the new release no longer has are deleted last. Only objects whose content changed are sent.

use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
  time::Duration,
};

use futures_util::{StreamExt, stream};
use md5::{Digest, Md5};
use reqwest::{Client, Method};
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle, actions::ListObjectsV2};

use crate::{OutputFile, Progress, PublishError};

/// Objects sent at the same time.
const CONCURRENCY: usize = 8;
const SIGNATURE_VALIDITY: Duration = Duration::from_secs(600);
/// What the last publish wrote, so the next one deletes only what it wrote and never other things in the bucket.
const MANIFEST: &str = ".yourdocs/manifest.json";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const SHORT: &str = "public, max-age=300";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Config {
  /// `None` for AWS, which is then reached at `https://s3.<region>.amazonaws.com`.
  pub endpoint: Option<String>,
  pub bucket: String,
  pub region: String,
  /// Where in the bucket the site lives, without slashes at the ends; empty for the root.
  pub prefix: String,
  pub access_key: String,
  pub secret_key: String,
  /// `<endpoint>/<bucket>/…` instead of `<bucket>.<endpoint>/…`: what MinIO and most other servers want.
  pub path_style: bool,
}

pub struct S3Target {
  bucket: Bucket,
  credentials: Credentials,
  http: Client,
  prefix: String,
}

/// One object to have in the bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Object {
  key: String,
  bytes: Vec<u8>,
  content_type: String,
  cache_control: &'static str,
  /// Where the bucket's website endpoint sends a reader who asks for this object.
  redirect: Option<String>,
  /// Assets go before the pages that refer to them.
  asset: bool,
}

impl S3Target {
  pub fn new(config: S3Config) -> Result<Self, PublishError> {
    let endpoint = config.endpoint.unwrap_or_else(|| format!("https://s3.{}.amazonaws.com", config.region));
    let url = endpoint.parse().map_err(|e| PublishError::Other(anyhow::anyhow!("S3_ENDPOINT is not a URL: {e}")))?;
    let style = if config.path_style { UrlStyle::Path } else { UrlStyle::VirtualHost };
    let bucket = Bucket::new(url, style, config.bucket, config.region).map_err(|e| PublishError::Other(anyhow::anyhow!("the S3 endpoint is not usable: {e}")))?;
    let http = Client::builder().timeout(Duration::from_secs(120)).build().map_err(|e| PublishError::Other(e.into()))?;

    Ok(Self {
      bucket,
      credentials: Credentials::new(config.access_key, config.secret_key),
      http,
      prefix: config.prefix.trim_matches('/').to_string(),
    })
  }

  /// What the bucket's website endpoint cannot do: redirect rules with wildcards or placeholders. Each entry says which rule.
  pub fn check(files: &[OutputFile]) -> Vec<String> {
    let Some(redirects) = files.iter().find(|f| f.path == "_redirects") else { return Vec::new() };

    String::from_utf8_lossy(&redirects.bytes)
      .lines()
      .map(str::trim)
      .filter(|l| !l.is_empty() && !l.starts_with('#'))
      .filter(|rule| rule.split_whitespace().next().is_some_and(|from| from.contains('*') || from.contains(':')))
      .map(|rule| format!("the redirect `{rule}` has a wildcard or placeholder, which S3 websites do not support"))
      .collect()
  }

  /// Sends the site: changed assets, then changed pages, then the deletion of what the previous publish wrote and this one does not.
  pub async fn publish(&self, files: &[OutputFile], progress: &Progress) -> Result<(), PublishError> {
    let objects: Vec<Arc<Object>> = plan(&self.prefix, files).into_iter().map(Arc::new).collect();
    let existing = self.list().await?;

    // A redirect has no content to compare, only a target in a header: it is always sent, and it is small.
    let changed = |asset: bool| {
      objects
        .iter()
        .filter(|o| o.asset == asset && (o.redirect.is_some() || existing.get(&o.key) != Some(&etag(&o.bytes))))
        .cloned()
        .collect::<Vec<_>>()
    };
    let (assets, pages) = (changed(true), changed(false));
    tracing::info!(assets = assets.len(), pages = pages.len(), unchanged = objects.len() - assets.len() - pages.len(), "uploading to S3");

    progress.start(assets.len() + pages.len());
    self.send_all(assets, progress).await?;
    self.send_all(pages, progress).await?;

    let keys: HashSet<String> = objects.iter().map(|o| o.key.clone()).collect();
    let previous = self.previous_keys().await?;
    let stale: Vec<String> = previous.into_iter().filter(|key| !keys.contains(key)).collect();
    // The futures own their data: borrowed closures would make this one too lifetime-generic to be spawned.
    let deleted: Vec<Result<(), PublishError>> = stream::iter(stale.into_iter().map(|key| self.delete(key))).buffer_unordered(CONCURRENCY).collect().await;
    deleted.into_iter().collect::<Result<(), _>>()?;

    let manifest = serde_json::to_vec(&objects.iter().map(|o| o.key.as_str()).collect::<Vec<_>>()).map_err(|e| PublishError::Other(e.into()))?;
    self
      .put(Arc::new(Object {
        key: self.key(MANIFEST),
        bytes: manifest,
        content_type: "application/json".into(),
        cache_control: "no-store",
        redirect: None,
        asset: false,
      }))
      .await
  }

  fn key(&self, path: &str) -> String {
    if self.prefix.is_empty() { path.to_string() } else { format!("{}/{path}", self.prefix) }
  }

  /// Every object under the prefix with its ETag. For a plain upload that is the MD5 of the content, which is what is compared.
  async fn list(&self) -> Result<HashMap<String, String>, PublishError> {
    let mut found = HashMap::new();
    let mut token: Option<String> = None;

    loop {
      let mut action = self.bucket.list_objects_v2(Some(&self.credentials));
      if !self.prefix.is_empty() {
        action.with_prefix(format!("{}/", self.prefix));
      }
      if let Some(token) = &token {
        action.with_continuation_token(token.clone());
      }
      let response = self.http.get(action.sign(SIGNATURE_VALIDITY)).send().await.map_err(request_error)?;
      let status = response.status();
      let text = response.text().await.map_err(request_error)?;
      if !status.is_success() {
        return Err(PublishError::Other(anyhow::anyhow!(
          "S3 answered {status} when listing the bucket: {}",
          text.chars().take(300).collect::<String>()
        )));
      }

      let page = ListObjectsV2::parse_response(&text).map_err(|e| PublishError::Other(anyhow::anyhow!("S3's list of objects is not understood: {e}")))?;
      found.extend(page.contents.into_iter().map(|o| (o.key, o.etag.trim_matches('"').to_string())));
      match page.next_continuation_token {
        Some(next) => token = Some(next),
        None => return Ok(found),
      }
    }
  }

  /// The keys the previous publish wrote, or none the first time.
  async fn previous_keys(&self) -> Result<Vec<String>, PublishError> {
    let key = self.key(MANIFEST);
    let action = self.bucket.get_object(Some(&self.credentials), &key);
    let response = self.http.get(action.sign(SIGNATURE_VALIDITY)).send().await.map_err(request_error)?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
      return Ok(Vec::new());
    }
    if !response.status().is_success() {
      return Err(PublishError::Other(anyhow::anyhow!("S3 answered {} when reading the previous publish's manifest", response.status())));
    }

    // A manifest that cannot be read only costs the deletions of this publish.
    Ok(serde_json::from_slice(&response.bytes().await.map_err(request_error)?).unwrap_or_default())
  }

  async fn send_all(&self, objects: Vec<Arc<Object>>, progress: &Progress) -> Result<(), PublishError> {
    let results: Vec<Result<(), PublishError>> = stream::iter(objects.into_iter().map(|object| async move {
      let result = self.put(object).await;
      progress.advance(1);
      result
    }))
    .buffer_unordered(CONCURRENCY)
    .collect()
    .await;

    results.into_iter().collect()
  }

  async fn put(&self, object: Arc<Object>) -> Result<(), PublishError> {
    let mut action = self.bucket.put_object(Some(&self.credentials), &object.key);
    action.headers_mut().insert("content-type", &object.content_type);
    action.headers_mut().insert("cache-control", object.cache_control);
    if let Some(location) = &object.redirect {
      action.headers_mut().insert("x-amz-website-redirect-location", location);
    }

    let mut request = self
      .http
      .request(Method::PUT, action.sign(SIGNATURE_VALIDITY))
      .header("content-type", &object.content_type)
      .header("cache-control", object.cache_control);
    if let Some(location) = &object.redirect {
      request = request.header("x-amz-website-redirect-location", location);
    }
    let response = request.body(object.bytes.clone()).send().await.map_err(request_error)?;

    if !response.status().is_success() {
      return Err(PublishError::Other(anyhow::anyhow!("S3 answered {} when storing {}", response.status(), object.key)));
    }

    Ok(())
  }

  async fn delete(&self, key: String) -> Result<(), PublishError> {
    let action = self.bucket.delete_object(Some(&self.credentials), &key);
    let response = self.http.delete(action.sign(SIGNATURE_VALIDITY)).send().await.map_err(request_error)?;

    // Already gone is as good as deleted.
    if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
      return Err(PublishError::Other(anyhow::anyhow!("S3 answered {} when deleting {key}", response.status())));
    }

    Ok(())
  }
}

fn request_error(error: reqwest::Error) -> PublishError {
  PublishError::Other(anyhow::anyhow!("cannot reach S3: {}", error.without_url()))
}

/// The ETag S3 gives a plain upload of these bytes.
fn etag(bytes: &[u8]) -> String {
  Md5::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The objects a site becomes: its files, and one empty object per redirect with the website redirect set.
fn plan(prefix: &str, files: &[OutputFile]) -> Vec<Object> {
  let key = |path: &str| if prefix.is_empty() { path.to_string() } else { format!("{prefix}/{path}") };
  let mut objects: Vec<Object> = files
    .iter()
    // The rules are objects of their own below; the file means nothing to S3.
    .filter(|f| f.path != "_redirects")
    .map(|file| {
      let extension = std::path::Path::new(&file.path).extension().and_then(|e| e.to_str()).unwrap_or_default();
      // Hashed names never change their content; everything else may.
      let hashed = file.path.starts_with("assets/") || file.path.starts_with("_a/");
      let page = matches!(extension, "html" | "md" | "xml" | "txt");

      Object {
        key: key(&file.path),
        bytes: file.bytes.clone(),
        content_type: mime_guess::from_path(&file.path).first_or_octet_stream().to_string(),
        cache_control: if hashed { IMMUTABLE } else { SHORT },
        redirect: None,
        asset: !page,
      }
    })
    .collect();

  let taken: HashSet<String> = objects.iter().map(|o| o.key.clone()).collect();
  if let Some(redirects) = files.iter().find(|f| f.path == "_redirects") {
    for rule in String::from_utf8_lossy(&redirects.bytes).lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
      let mut parts = rule.split_whitespace();
      let (Some(from), Some(to)) = (parts.next(), parts.next()) else { continue };
      // A folder is served from its `index.html`; that is the object a website endpoint looks for.
      let path = from.trim_start_matches('/');
      let path = if path.is_empty() || path.ends_with('/') { format!("{path}index.html") } else { path.to_string() };
      let redirect_key = key(&path);

      // A real file at the same address wins: the rule would hide a page.
      if !taken.contains(&redirect_key) {
        objects.push(Object {
          key: redirect_key,
          bytes: Vec::new(),
          content_type: "text/html".into(),
          cache_control: SHORT,
          redirect: Some(to.to_string()),
          asset: false,
        });
      }
    }
  }

  objects
}

#[cfg(test)]
mod tests {
  use super::*;

  fn file(path: &str, text: &str) -> OutputFile {
    OutputFile {
      path: path.into(),
      bytes: text.as_bytes().to_vec(),
    }
  }

  #[test]
  fn files_and_redirects_become_objects_with_the_right_headers() {
    let files = vec![
      file("index.html", "home"),
      file("guide/index.html", "guide"),
      file("assets/site-abc.css", "body{}"),
      file("_a/1-480.webp", "img"),
      file("pagefind/pagefind.js", "js"),
      file("_redirects", "/ /guide/ 302\n/old/ /guide/ 301\n/old-page /guide/ 301\n# a comment\n/guide/ /elsewhere/ 301"),
    ];

    let objects = plan("docs", &files);
    let by_key: HashMap<&str, &Object> = objects.iter().map(|o| (o.key.as_str(), o)).collect();

    // (key, asset, cache control, redirect)
    for (key, asset, cache, redirect) in [
      ("docs/guide/index.html", false, SHORT, None),
      ("docs/assets/site-abc.css", true, IMMUTABLE, None),
      ("docs/_a/1-480.webp", true, IMMUTABLE, None),
      ("docs/pagefind/pagefind.js", true, SHORT, None),
      ("docs/old/index.html", false, SHORT, Some("/guide/")),
      ("docs/old-page", false, SHORT, Some("/guide/")),
    ] {
      let object = by_key.get(key).unwrap_or_else(|| panic!("missing {key}"));
      assert_eq!((object.asset, object.cache_control, object.redirect.as_deref()), (asset, cache, redirect), "{key}");
    }
    assert_eq!(by_key["docs/assets/site-abc.css"].content_type, "text/css");
    // The root redirect replaces nothing; the rule for a path that has a page is dropped; the rules file itself is not uploaded.
    assert_eq!(by_key["docs/index.html"].redirect.as_deref(), None, "a real page wins over a rule");
    assert_eq!(by_key["docs/guide/index.html"].redirect, None);
    assert!(!by_key.contains_key("docs/_redirects"));
  }

  #[test]
  fn wildcard_redirects_are_reported() {
    let files = [file("_redirects", "/a/ /b/ 301\n/blog/* /news/:splat 301\n/c/:id /d/:id 301")];

    let problems = S3Target::check(&files);

    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems.iter().all(|p| p.contains("wildcard")));
    assert!(S3Target::check(&[file("index.html", "x")]).is_empty());
  }

  #[test]
  fn the_etag_is_the_md5_of_the_content() {
    assert_eq!(etag(b"hello"), "5d41402abc4b2a76b9719d911017c592");
  }

  /// An S3 that keeps objects in memory: just enough of the protocol (object PUT, GET and DELETE, and listing by prefix) to see what a
  /// publish sends.
  mod fake {
    use std::{
      collections::BTreeMap,
      sync::{Arc, Mutex},
    };

    use axum::{
      Router,
      body::Bytes,
      extract::State,
      http::{HeaderMap, Method, StatusCode, Uri},
      response::{IntoResponse, Response},
    };

    #[derive(Clone, Default)]
    pub struct Stored {
      pub bytes: Vec<u8>,
      pub redirect: Option<String>,
      pub cache_control: String,
    }

    #[derive(Default)]
    pub struct S3 {
      pub objects: BTreeMap<String, Stored>,
      /// Keys of every PUT, in order.
      pub puts: Vec<String>,
      pub deletes: Vec<String>,
    }

    pub async fn start() -> (String, Arc<Mutex<S3>>) {
      let state = Arc::new(Mutex::new(S3::default()));
      let app = Router::new().fallback(handle).with_state(state.clone());
      let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
      let address = listener.local_addr().unwrap();
      tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

      (format!("http://{address}"), state)
    }

    async fn handle(State(state): State<Arc<Mutex<S3>>>, method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
      // Path style: `/<bucket>/<key>`.
      let path = uri.path().trim_start_matches('/');
      let key = path.split_once('/').map(|(_, key)| key).unwrap_or_default().to_string();
      let query = uri.query().unwrap_or_default();
      let mut s3 = state.lock().unwrap();

      match method {
        Method::PUT => {
          let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).map(String::from);
          s3.puts.push(key.clone());
          s3.objects.insert(
            key,
            Stored {
              bytes: body.to_vec(),
              redirect: header("x-amz-website-redirect-location"),
              cache_control: header("cache-control").unwrap_or_default(),
            },
          );
          StatusCode::OK.into_response()
        }
        Method::DELETE => {
          s3.deletes.push(key.clone());
          s3.objects.remove(&key);
          StatusCode::NO_CONTENT.into_response()
        }
        Method::GET if query.contains("list-type=2") => {
          let prefix = query.split('&').find_map(|p| p.strip_prefix("prefix=")).map(|p| p.replace("%2F", "/")).unwrap_or_default();
          let contents: String = s3
            .objects
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(key, o)| {
              let etag = super::etag(&o.bytes);
              format!(
                "<Contents><Key>{key}</Key><LastModified>2020-01-01T00:00:00.000Z</LastModified><ETag>&quot;{etag}&quot;</ETag><Size>{}</Size><StorageClass>STANDARD</StorageClass></Contents>",
                o.bytes.len()
              )
            })
            .collect();

          (
            [("content-type", "application/xml")],
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"><Name>docs</Name><Prefix>{prefix}</Prefix><KeyCount>1</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated>{contents}</ListBucketResult>"),
          )
            .into_response()
        }
        Method::GET => match s3.objects.get(&key) {
          Some(object) => object.bytes.clone().into_response(),
          None => (StatusCode::NOT_FOUND, "<Error><Code>NoSuchKey</Code></Error>").into_response(),
        },
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
      }
    }
  }

  async fn target(prefix: &str) -> (S3Target, Arc<std::sync::Mutex<fake::S3>>) {
    let (url, state) = fake::start().await;
    let target = S3Target::new(S3Config {
      endpoint: Some(url),
      bucket: "docs".into(),
      region: "us-east-1".into(),
      prefix: prefix.into(),
      access_key: "key".into(),
      secret_key: "secret".into(),
      path_style: true,
    })
    .unwrap();

    (target, state)
  }

  #[tokio::test]
  async fn only_changed_objects_are_sent_assets_first_and_stale_ones_are_deleted_last() {
    let (target, s3) = target("site").await;
    let site = |guide: &str, extra: bool| {
      let mut files = vec![
        file("index.html", "home"),
        file("guide/index.html", guide),
        file("assets/app-1.js", "js"),
        file("_redirects", "/old/ /guide/ 301"),
      ];
      if extra {
        files.push(file("gone/index.html", "soon gone"));
      }
      files
    };

    target.publish(&site("v1", true), &Progress::none()).await.unwrap();
    {
      let state = s3.lock().unwrap();
      let order: Vec<&str> = state.puts.iter().map(String::as_str).collect();
      assert_eq!(order[0], "site/assets/app-1.js", "assets go before the pages that use them: {order:?}");
      assert_eq!(order.last(), Some(&"site/.yourdocs/manifest.json"), "the manifest is last");
      assert_eq!(state.objects["site/old/index.html"].redirect.as_deref(), Some("/guide/"));
      assert!(state.objects["site/old/index.html"].bytes.is_empty());
      assert_eq!(state.objects["site/assets/app-1.js"].cache_control, IMMUTABLE);
    }

    // A page changed and one was removed: one page is sent again (and the redirect, which is always sent), the removed one is deleted
    // after everything is in place, and the unchanged files are left alone.
    let before = s3.lock().unwrap().puts.len();
    target.publish(&site("v2", false), &Progress::none()).await.unwrap();

    let state = s3.lock().unwrap();
    let sent: Vec<&str> = state.puts[before..].iter().map(String::as_str).collect();
    assert_eq!(sent, ["site/guide/index.html", "site/old/index.html", "site/.yourdocs/manifest.json"]);
    assert_eq!(state.deletes, ["site/gone/index.html"]);
    assert!(!state.objects.contains_key("site/gone/index.html"));
    assert!(state.objects.contains_key("site/index.html"));
  }

  #[tokio::test]
  async fn objects_the_site_never_wrote_are_never_deleted() {
    let (target, s3) = target("").await;
    s3.lock().unwrap().objects.insert("someone-elses.txt".into(), fake::Stored::default());

    target.publish(&[file("index.html", "home")], &Progress::none()).await.unwrap();
    target.publish(&[file("other/index.html", "other")], &Progress::none()).await.unwrap();

    let state = s3.lock().unwrap();
    assert!(state.objects.contains_key("someone-elses.txt"));
    assert_eq!(state.deletes, ["index.html"], "only what an earlier publish wrote");
  }
}
