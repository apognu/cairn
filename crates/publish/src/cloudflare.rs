//! The Cloudflare Workers static assets target: the site is uploaded through Cloudflare's API as a version of a Worker, so a build can be
//! looked at before it goes live and then deployed exactly as it was uploaded.
//!
//! Files are identified by a hash of their content. Cloudflare says which of them it does not have yet, so a publish sends only what
//! changed since the last one.

use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, multipart};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{OutputFile, PublishError};

/// What Cloudflare accepts of a Worker's static assets (paid plans); checked before anything is uploaded.
const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;
const MAX_FILES: usize = 100_000;
/// `_redirects`: rules without placeholders, and rules with them.
const MAX_STATIC_REDIRECTS: usize = 2_000;
const MAX_DYNAMIC_REDIRECTS: usize = 100;
/// The Worker's own script, which does nothing: requests are answered from the assets before it would run.
const WORKER_SCRIPT: &str = "export default { fetch: (request, env) => env.ASSETS.fetch(request) };";
const COMPATIBILITY_DATE: &str = "2025-09-01";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudflareConfig {
  pub account_id: String,
  pub api_token: String,
  /// The Worker that serves the site.
  pub worker: String,
  /// `https://api.cloudflare.com/client/v4`; other values are for tests.
  pub api_url: String,
}

pub struct CloudflareTarget {
  config: CloudflareConfig,
  http: Client,
}

/// A build that was uploaded to Cloudflare: its version, and where it can be looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
  pub id: String,
  pub preview_url: Option<String>,
}

#[derive(Deserialize)]
struct Envelope<T> {
  result: Option<T>,
  /// `null` on success.
  errors: Option<Vec<ApiError>>,
}

#[derive(Deserialize)]
struct ApiError {
  message: String,
}

#[derive(Deserialize)]
struct Session {
  jwt: String,
  /// `null`, not an empty list, when Cloudflare has every file.
  buckets: Option<Vec<Vec<String>>>,
}

#[derive(Deserialize)]
struct Uploaded {
  jwt: Option<String>,
}

#[derive(Deserialize)]
struct VersionResult {
  id: String,
}

#[derive(Deserialize)]
struct Subdomain {
  subdomain: String,
}

impl CloudflareTarget {
  pub fn new(config: CloudflareConfig) -> Result<Self, PublishError> {
    let http = Client::builder()
      .timeout(std::time::Duration::from_secs(120))
      .user_agent("yourdocs")
      .build()
      .map_err(|e| PublishError::Other(e.into()))?;

    Ok(Self { config, http })
  }

  fn url(&self, path: &str) -> String {
    format!("{}/accounts/{}{path}", self.config.api_url, self.config.account_id)
  }

  fn script(&self, path: &str) -> String {
    self.url(&format!("/workers/scripts/{}{path}", self.config.worker))
  }

  /// What would stop an upload: files over the size limit, too many files, too many redirect rules. Each entry says what to change.
  pub fn check(files: &[OutputFile]) -> Vec<String> {
    let mut problems = Vec::new();

    for file in files.iter().filter(|f| f.bytes.len() > MAX_FILE_BYTES) {
      problems.push(format!(
        "{} is {} MiB; Cloudflare accepts files up to 25 MiB. Host large files elsewhere.",
        file.path,
        file.bytes.len() / (1024 * 1024)
      ));
    }
    if files.len() > MAX_FILES {
      problems.push(format!("the site has {} files; Cloudflare accepts up to {MAX_FILES} per Worker", files.len()));
    }

    if let Some(redirects) = files.iter().find(|f| f.path == "_redirects") {
      let text = String::from_utf8_lossy(&redirects.bytes);
      let rules = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
      let (dynamic, fixed): (Vec<_>, Vec<_>) = rules.partition(|rule| rule.split_whitespace().next().is_some_and(|from| from.contains('*') || from.contains(':')));

      if fixed.len() > MAX_STATIC_REDIRECTS {
        problems.push(format!("the site has {} redirects; Cloudflare accepts {MAX_STATIC_REDIRECTS} fixed ones", fixed.len()));
      }
      if dynamic.len() > MAX_DYNAMIC_REDIRECTS {
        problems.push(format!(
          "the site has {} redirects with wildcards or placeholders; Cloudflare accepts {MAX_DYNAMIC_REDIRECTS}",
          dynamic.len()
        ));
      }
      if fixed.len() * 10 > MAX_STATIC_REDIRECTS * 9 {
        tracing::warn!(rules = fixed.len(), limit = MAX_STATIC_REDIRECTS, "the site is close to Cloudflare's limit of fixed redirects");
      }
    }

    problems
  }

  /// Uploads what Cloudflare does not have yet and makes a version of the Worker from it, without deploying it.
  pub async fn upload(&self, files: &[OutputFile]) -> Result<Version, PublishError> {
    let completion = self.upload_assets(files).await?;
    let id = self.create_version(&completion, files, false).await?;

    Ok(Version {
      preview_url: self.preview_url(&id).await,
      id,
    })
  }

  /// Uploads a build and makes it live in one step. The first publish creates the Worker, which has no versions to deploy yet.
  pub async fn publish(&self, files: &[OutputFile]) -> Result<Version, PublishError> {
    let completion = self.upload_assets(files).await?;

    match self.create_version(&completion, files, false).await {
      Ok(id) => {
        self.deploy(&id).await?;
        Ok(Version { preview_url: None, id })
      }
      // The Worker does not exist yet: uploading its script creates it and deploys it.
      Err(PublishError::Other(error)) if error.to_string().contains("answered 404") => {
        let id = self.create_version(&completion, files, true).await?;
        Ok(Version { preview_url: None, id })
      }
      Err(error) => Err(error),
    }
  }

  /// Makes a version that was uploaded earlier the live one, exactly as it was.
  pub async fn deploy(&self, version: &str) -> Result<(), PublishError> {
    let body = json!({ "strategy": "percentage", "versions": [{ "percentage": 100, "version_id": version }] });
    let response = self
      .http
      .post(self.script("/deployments"))
      .bearer_auth(&self.config.api_token)
      .json(&body)
      .send()
      .await
      .map_err(request_error)?;

    read::<Value>(response).await.map(|_| ())
  }

  /// Hashes the files, asks Cloudflare which it lacks, sends those, and returns the token that stands for the whole set.
  async fn upload_assets(&self, files: &[OutputFile]) -> Result<String, PublishError> {
    let assets: Vec<(&OutputFile, String)> = files.iter().filter(|f| !is_config(&f.path)).map(|f| (f, hash(f))).collect();
    let manifest: serde_json::Map<String, Value> = assets
      .iter()
      .map(|(file, hash)| (format!("/{}", file.path), json!({ "hash": hash, "size": file.bytes.len() })))
      .collect();

    let response = self
      .http
      .post(self.script("/assets-upload-session"))
      .bearer_auth(&self.config.api_token)
      .json(&json!({ "manifest": manifest }))
      .send()
      .await
      .map_err(request_error)?;
    let session: Session = read(response).await?;

    // Nothing to send: Cloudflare has every file already, and the session token is the completion token.
    let buckets = session.buckets.unwrap_or_default();
    if buckets.is_empty() {
      return Ok(session.jwt);
    }

    let by_hash: HashMap<&str, &OutputFile> = assets.iter().map(|(file, hash)| (hash.as_str(), *file)).collect();
    let mut completion = None;

    for bucket in &buckets {
      let mut form = multipart::Form::new();
      for wanted in bucket {
        let file = by_hash
          .get(wanted.as_str())
          .ok_or_else(|| PublishError::Other(anyhow::anyhow!("Cloudflare asked for a file that is not in the site: {wanted}")))?;
        let part = multipart::Part::text(STANDARD.encode(&file.bytes))
          .file_name(wanted.clone())
          .mime_str(mime_guess::from_path(&file.path).first_or_octet_stream().as_ref())
          .map_err(|e| PublishError::Other(e.into()))?;
        form = form.part(wanted.clone(), part);
      }

      let response = self
        .http
        .post(self.url("/workers/assets/upload?base64=true"))
        .bearer_auth(&session.jwt)
        .multipart(form)
        .send()
        .await
        .map_err(request_error)?;
      let uploaded: Uploaded = read(response).await?;
      // The last bucket's answer carries the completion token.
      completion = uploaded.jwt.or(completion);
    }

    completion.ok_or_else(|| PublishError::Other(anyhow::anyhow!("Cloudflare did not confirm the upload")))
  }

  /// Makes a version of the Worker serving the uploaded assets. With `create`, uploads the script itself instead, which creates the
  /// Worker and deploys it at once.
  async fn create_version(&self, completion: &str, files: &[OutputFile], create: bool) -> Result<String, PublishError> {
    let text = |name: &str| files.iter().find(|f| f.path == name).map(|f| String::from_utf8_lossy(&f.bytes).into_owned());
    let mut config = json!({ "html_handling": "auto-trailing-slash", "not_found_handling": "404-page" });
    for name in ["_redirects", "_headers"] {
      if let Some(content) = text(name) {
        config[name] = Value::String(content);
      }
    }
    let metadata = json!({
      "main_module": "worker.js",
      "compatibility_date": COMPATIBILITY_DATE,
      "bindings": [{ "name": "ASSETS", "type": "assets" }],
      "assets": { "jwt": completion, "config": config },
    });
    let form = multipart::Form::new()
      .part(
        "metadata",
        multipart::Part::text(metadata.to_string()).mime_str("application/json").map_err(|e| PublishError::Other(e.into()))?,
      )
      .part(
        "worker.js",
        multipart::Part::text(WORKER_SCRIPT)
          .file_name("worker.js")
          .mime_str("application/javascript+module")
          .map_err(|e| PublishError::Other(e.into()))?,
      );

    let request = if create { self.http.put(self.script("")) } else { self.http.post(self.script("/versions")) };
    let response = request.bearer_auth(&self.config.api_token).multipart(form).send().await.map_err(request_error)?;
    let version: VersionResult = read(response).await?;

    Ok(version.id)
  }

  /// The address a version can be looked at before it is deployed: its own host under the account's `workers.dev` name. Not every account
  /// has one, and a preview without an address is still a version that can be deployed.
  async fn preview_url(&self, version: &str) -> Option<String> {
    self.enable_previews().await;

    let response = self.http.get(self.url("/workers/subdomain")).bearer_auth(&self.config.api_token).send().await.ok()?;
    let subdomain: Subdomain = read(response).await.ok()?;

    Some(format!("https://{}-{}.{}.workers.dev", version.split('-').next()?, self.config.worker, subdomain.subdomain))
  }
}

impl CloudflareTarget {
  /// A Worker answers its version addresses only when preview URLs are enabled for it, and they are off by default. Failing to switch
  /// them on (a token without the right) only leaves the preview without a working address.
  async fn enable_previews(&self) {
    let url = self.script("/subdomain");
    let Ok(response) = self.http.get(&url).bearer_auth(&self.config.api_token).send().await else {
      return;
    };
    let Ok(settings) = read::<Value>(response).await else { return };
    if settings["previews_enabled"].as_bool() == Some(true) {
      return;
    }

    let body = json!({ "enabled": settings["enabled"].as_bool().unwrap_or(true), "previews_enabled": true });
    match self.http.post(&url).bearer_auth(&self.config.api_token).json(&body).send().await {
      Ok(response) if response.status().is_success() => tracing::info!(worker = %self.config.worker, "preview URLs enabled for the Worker"),
      _ => tracing::warn!(worker = %self.config.worker, "could not enable preview URLs for the Worker; its previews will not open"),
    }
  }
}

/// `_redirects` and `_headers` are sent as settings of the Worker, not as assets.
fn is_config(path: &str) -> bool {
  matches!(path, "_redirects" | "_headers")
}

/// The identity of a file's content for Cloudflare: 32 hex characters of the BLAKE3 hash of its Base64 text and extension.
fn hash(file: &OutputFile) -> String {
  let extension = std::path::Path::new(&file.path).extension().and_then(|e| e.to_str()).unwrap_or_default();
  let digest = blake3::hash(format!("{}{extension}", STANDARD.encode(&file.bytes)).as_bytes());

  digest.to_hex()[..32].to_string()
}

fn request_error(error: reqwest::Error) -> PublishError {
  PublishError::Other(anyhow::anyhow!("cannot reach Cloudflare: {}", error.without_url()))
}

/// The `result` of a Cloudflare answer, or the reason it gave.
async fn read<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T, PublishError> {
  let status = response.status();
  let bytes = response.bytes().await.map_err(request_error)?;
  let envelope: Result<Envelope<T>, _> = serde_json::from_slice(&bytes);

  match envelope {
    Ok(Envelope { result: Some(result), .. }) if status.is_success() => Ok(result),
    Ok(envelope) => {
      let reasons: Vec<_> = envelope.errors.iter().flatten().map(|e| e.message.as_str()).collect();
      Err(PublishError::Other(anyhow::anyhow!("Cloudflare answered {status}: {}", reasons.join("; "))))
    }
    Err(error) => Err(PublishError::Other(anyhow::anyhow!(
      "Cloudflare answered {status} with something that is not understood ({error}): {}",
      String::from_utf8_lossy(&bytes).chars().take(300).collect::<String>()
    ))),
  }
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
  fn a_hash_depends_on_the_content_and_the_extension() {
    let a = hash(&file("a.html", "<p>a</p>"));

    assert_eq!(a.len(), 32);
    assert_eq!(a, hash(&file("other/place.html", "<p>a</p>")));
    assert_ne!(a, hash(&file("a.html", "<p>b</p>")));
    assert_ne!(a, hash(&file("a.css", "<p>a</p>")));
  }

  #[test]
  fn what_cloudflare_would_refuse_is_found_before_uploading() {
    let redirects = |fixed: usize, dynamic: usize| {
      let rules: Vec<String> = (0..fixed)
        .map(|i| format!("/old-{i}/ /new-{i}/ 301"))
        .chain((0..dynamic).map(|i| format!("/blog-{i}/* /news/:splat 301")))
        .collect();
      file("_redirects", &rules.join("\n"))
    };
    let big = OutputFile {
      path: "_a/video.mp4".into(),
      bytes: vec![0; MAX_FILE_BYTES + 1],
    };

    // (files, number of problems, a word every problem must contain)
    for (files, count, word) in [
      (vec![file("index.html", "x"), redirects(2_000, 100)], 0, ""),
      (vec![big], 1, "_a/video.mp4"),
      (vec![redirects(2_001, 0)], 1, "fixed"),
      (vec![redirects(0, 101)], 1, "wildcards"),
    ] {
      let problems = CloudflareTarget::check(&files);

      assert_eq!(problems.len(), count, "{problems:?}");
      assert!(problems.iter().all(|p| p.contains(word)), "{problems:?}");
    }
  }

  /// A Cloudflare that keeps what it was sent, to see what a publish does.
  mod fake {
    use std::{
      collections::HashSet,
      sync::{Arc, Mutex},
    };

    use axum::{
      Json, Router,
      extract::{Multipart, State},
      http::StatusCode,
      routing::{get, post},
    };

    #[derive(Default)]
    pub struct Cloudflare {
      /// Hashes of the files Cloudflare holds.
      pub known: HashSet<String>,
      /// What the current upload session still waits for.
      pub waiting: HashSet<String>,
      /// Every upload request: the hashes it carried.
      pub uploads: Vec<Vec<String>>,
      pub worker_exists: bool,
      pub versions: Vec<String>,
      pub deployed: Vec<String>,
      /// The `_redirects` text the last version was made with.
      pub redirects: Option<String>,
    }

    type Shared = Arc<Mutex<Cloudflare>>;

    pub async fn start() -> (String, Shared) {
      let state = Shared::default();
      let app = Router::new()
        .route("/accounts/acct/workers/scripts/site/assets-upload-session", post(session))
        .route("/accounts/acct/workers/assets/upload", post(upload))
        .route("/accounts/acct/workers/scripts/site/versions", post(version))
        .route("/accounts/acct/workers/scripts/site", axum::routing::put(create))
        .route("/accounts/acct/workers/scripts/site/deployments", post(deployment))
        .route(
          "/accounts/acct/workers/subdomain",
          get(|| async { Json(serde_json::json!({ "success": true, "result": { "subdomain": "acme" } })) }),
        )
        .with_state(state.clone());
      let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
      let address = listener.local_addr().unwrap();
      tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

      (format!("http://{address}"), state)
    }

    async fn session(State(state): State<Shared>, Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
      let mut cloudflare = state.lock().unwrap();
      let wanted: HashSet<String> = body["manifest"].as_object().unwrap().values().map(|f| f["hash"].as_str().unwrap().to_string()).collect();
      let missing: Vec<String> = wanted.iter().filter(|h| !cloudflare.known.contains(*h)).cloned().collect();
      cloudflare.waiting = missing.iter().cloned().collect();

      // Two files to a bucket, as Cloudflare groups them.
      let buckets: Vec<Vec<String>> = missing.chunks(2).map(<[String]>::to_vec).collect();
      let jwt = if buckets.is_empty() { "complete" } else { "session" };

      Json(serde_json::json!({ "success": true, "result": { "jwt": jwt, "buckets": buckets } }))
    }

    async fn upload(State(state): State<Shared>, headers: axum::http::HeaderMap, mut form: Multipart) -> (StatusCode, Json<serde_json::Value>) {
      assert_eq!(headers["authorization"], "Bearer session", "an upload carries the session token");
      let mut sent = Vec::new();

      while let Some(field) = form.next_field().await.unwrap() {
        let hash = field.name().unwrap().to_string();
        let body = field.text().await.unwrap();
        // The body is the file in Base64.
        assert!(<base64::engine::GeneralPurpose as base64::Engine>::decode(&base64::engine::general_purpose::STANDARD, &body).is_ok());
        sent.push(hash);
      }

      let mut cloudflare = state.lock().unwrap();
      for hash in &sent {
        cloudflare.waiting.remove(hash);
        cloudflare.known.insert(hash.clone());
      }
      cloudflare.uploads.push(sent);

      if cloudflare.waiting.is_empty() {
        (StatusCode::CREATED, Json(serde_json::json!({ "success": true, "result": { "jwt": "complete" } })))
      } else {
        (StatusCode::ACCEPTED, Json(serde_json::json!({ "success": true, "result": {} })))
      }
    }

    async fn script_form(state: &Shared, mut form: Multipart) {
      while let Some(field) = form.next_field().await.unwrap() {
        if field.name() == Some("metadata") {
          let metadata: serde_json::Value = serde_json::from_str(&field.text().await.unwrap()).unwrap();
          assert_eq!(metadata["assets"]["jwt"], "complete", "the version is made from the completed upload");
          state.lock().unwrap().redirects = metadata["assets"]["config"]["_redirects"].as_str().map(String::from);
        }
      }
    }

    async fn version(State(state): State<Shared>, form: Multipart) -> (StatusCode, Json<serde_json::Value>) {
      if !state.lock().unwrap().worker_exists {
        return (
          StatusCode::NOT_FOUND,
          Json(serde_json::json!({ "success": false, "errors": [{ "code": 10007, "message": "This Worker does not exist on your account." }] })),
        );
      }
      script_form(&state, form).await;

      let mut cloudflare = state.lock().unwrap();
      let id = format!("{:08x}-0000-4000-8000-000000000000", cloudflare.versions.len() + 1);
      cloudflare.versions.push(id.clone());

      (StatusCode::OK, Json(serde_json::json!({ "success": true, "result": { "id": id } })))
    }

    async fn create(State(state): State<Shared>, form: Multipart) -> Json<serde_json::Value> {
      script_form(&state, form).await;
      let mut cloudflare = state.lock().unwrap();
      cloudflare.worker_exists = true;
      cloudflare.deployed.push("created".into());

      Json(serde_json::json!({ "success": true, "result": { "id": "created" } }))
    }

    async fn deployment(State(state): State<Shared>, Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
      assert_eq!(body["versions"][0]["percentage"], 100);
      state.lock().unwrap().deployed.push(body["versions"][0]["version_id"].as_str().unwrap().to_string());

      Json(serde_json::json!({ "success": true, "result": {} }))
    }
  }

  async fn target() -> (CloudflareTarget, std::sync::Arc<std::sync::Mutex<fake::Cloudflare>>) {
    let (url, state) = fake::start().await;
    let target = CloudflareTarget::new(CloudflareConfig {
      account_id: "acct".into(),
      api_token: "token".into(),
      worker: "site".into(),
      api_url: url,
    })
    .unwrap();

    (target, state)
  }

  #[tokio::test]
  async fn only_new_files_are_uploaded_and_the_first_publish_creates_the_worker() {
    let (target, cloudflare) = target().await;
    let site = |guide: &str| {
      vec![
        file("index.html", "home"),
        file("guide/index.html", guide),
        file("a.css", "body{}"),
        file("_redirects", "/old/ /new/ 301"),
      ]
    };

    // The first publish: the Worker does not exist, so it is created, and every file goes up (the `_redirects` file is a setting).
    target.publish(&site("v1")).await.unwrap();
    {
      let state = cloudflare.lock().unwrap();
      assert_eq!(state.uploads.iter().map(Vec::len).sum::<usize>(), 3);
      assert_eq!(state.deployed, ["created"]);
      assert_eq!(state.redirects.as_deref(), Some("/old/ /new/ 301"));
    }

    // One page changed: one file goes up, and the new version is the deployed one.
    target.publish(&site("v2")).await.unwrap();
    {
      let state = cloudflare.lock().unwrap();
      assert_eq!(
        state.uploads.iter().map(Vec::len).collect::<Vec<_>>(),
        [2, 1, 1],
        "the first publish sent two buckets, the second one file"
      );
      assert_eq!(state.deployed.last(), state.versions.last());
    }

    // Nothing changed: nothing goes up, a version is still made.
    target.publish(&site("v2")).await.unwrap();
    let state = cloudflare.lock().unwrap();
    assert_eq!(state.uploads.len(), 3);
    assert_eq!(state.versions.len(), 2);
  }

  #[tokio::test]
  async fn a_preview_is_not_deployed_until_it_is_promoted() {
    let (target, cloudflare) = target().await;
    cloudflare.lock().unwrap().worker_exists = true;

    let preview = target.upload(&[file("index.html", "new")]).await.unwrap();

    assert_eq!(preview.preview_url.as_deref(), Some("https://00000001-site.acme.workers.dev"));
    assert!(cloudflare.lock().unwrap().deployed.is_empty(), "a preview is not the site");

    target.deploy(&preview.id).await.unwrap();

    assert_eq!(cloudflare.lock().unwrap().deployed, [preview.id], "promoting deploys the version that was looked at");
  }

  #[tokio::test]
  async fn a_preview_needs_a_worker_to_make_a_version_of() {
    let (target, _) = target().await;

    let error = target.upload(&[file("index.html", "new")]).await.expect_err("no Worker yet");

    assert!(error.to_string().contains("does not exist"), "{error}");
  }

  fn walk(root: &std::path::Path, dir: &std::path::Path) -> Vec<OutputFile> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
      let path = entry.path();
      if path.is_dir() {
        files.extend(walk(root, &path));
      } else {
        files.push(OutputFile {
          path: path.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
          bytes: std::fs::read(&path).unwrap(),
        });
      }
    }
    files
  }

  /// Against a real account: `cargo test -p yourdocs-publish live_cloudflare -- --ignored --nocapture` with the `CLOUDFLARE_*` variables set.
  /// Uploads a one-page site as a version and does not deploy it.
  #[tokio::test]
  #[ignore = "needs a Cloudflare account"]
  async fn live_cloudflare_accepts_an_upload() {
    let var = |name| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    let target = CloudflareTarget::new(CloudflareConfig {
      account_id: var("CLOUDFLARE_ACCOUNT_ID"),
      api_token: var("CLOUDFLARE_API_TOKEN"),
      worker: var("CLOUDFLARE_WORKER"),
      api_url: "https://api.cloudflare.com/client/v4".into(),
    })
    .unwrap();

    // `LIVE_SITE_DIR`: a built release folder, to see a whole site go up (many files, several buckets).
    let files = match std::env::var("LIVE_SITE_DIR") {
      Ok(dir) => walk(std::path::Path::new(&dir), std::path::Path::new(&dir)),
      Err(_) => vec![file("index.html", "<h1>yourdocs smoke test</h1>"), file("404.html", "not found"), file("_redirects", "/old/ /new/ 301")],
    };
    let version = target.upload(&files).await;

    // The version is in the failure message when it does not work, and in the Worker's dashboard when it does.
    version.unwrap();
  }
}
