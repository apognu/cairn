//! The Cloudflare Pages target (the older product, still used by many projects): the site is a direct-upload deployment of a Pages project.
//!
//! Like Workers it identifies files by hash and sends only what Pages lacks. Unlike Workers, a preview is a deployment of another branch,
//! and Pages cannot make that exact deployment the production one: publishing builds the site again for the production branch.

use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, multipart};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
  OutputFile, Progress, PublishError,
  cloudflare::{check_limits, hash, is_config, read, read_ok, request_error},
};

/// What Pages accepts of a project's files on the free plan, the stricter of its plans.
pub(crate) const MAX_FILES: usize = 20_000;
/// A bucket of uploads is sent when it holds this much (as Base64 text) or this many files.
const BUCKET_BYTES: usize = 20 * 1024 * 1024;
const BUCKET_FILES: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagesConfig {
  pub account_id: String,
  pub api_token: cairn_core::Secret,
  pub project: String,
  /// The branch whose deployments are the site (`main` for a project created here).
  pub production_branch: String,
  /// The branch previews are deployed to.
  pub preview_branch: String,
  /// `https://api.cloudflare.com/client/v4`; other values are for tests.
  pub api_url: String,
}

pub struct PagesTarget {
  config: PagesConfig,
  http: Client,
}

#[derive(Deserialize)]
struct Token {
  jwt: String,
}

#[derive(Deserialize)]
struct Deployment {
  id: String,
  url: Option<String>,
}

impl PagesTarget {
  pub fn new(config: PagesConfig) -> Result<Self, PublishError> {
    let http = Client::builder()
      .timeout(std::time::Duration::from_secs(120))
      .user_agent("cairn")
      .build()
      .map_err(|e| PublishError::Other(e.into()))?;

    Ok(Self { config, http })
  }

  /// An address of the project (`path` starts with a slash), or of the list of projects when `path` is empty.
  fn project(&self, path: &str) -> String {
    let name = if path.is_empty() { String::new() } else { format!("/{}", self.config.project) };

    format!("{}/accounts/{}/pages/projects{name}{path}", self.config.api_url, self.config.account_id)
  }

  /// What would make Pages refuse these files; the same limits as Workers, with fewer files.
  pub fn check(files: &[OutputFile]) -> Vec<String> {
    check_limits(files, MAX_FILES)
  }

  /// Deploys the site to the production branch: it is the live site as soon as the deployment is ready.
  pub async fn publish(&self, files: &[OutputFile], progress: &Progress) -> Result<String, PublishError> {
    Ok(self.deploy(files, &self.config.production_branch, progress).await?.id)
  }

  /// Deploys the site to the preview branch: a deployment of its own, with an address, that is not the site.
  pub async fn upload(&self, files: &[OutputFile], progress: &Progress) -> Result<(String, Option<String>), PublishError> {
    let deployment = self.deploy(files, &self.config.preview_branch, progress).await?;

    Ok((deployment.id, deployment.url))
  }

  async fn deploy(&self, files: &[OutputFile], branch: &str, progress: &Progress) -> Result<Deployment, PublishError> {
    let assets: Vec<(&OutputFile, String)> = files.iter().filter(|f| !is_config(&f.path)).map(|f| (f, hash(f))).collect();
    self.upload_assets(&assets, progress).await?;

    let manifest: HashMap<String, &str> = assets.iter().map(|(file, hash)| (format!("/{}", file.path), hash.as_str())).collect();
    let mut form = multipart::Form::new()
      .text("manifest", serde_json::to_string(&manifest).map_err(|e| PublishError::Other(e.into()))?)
      .text("branch", branch.to_string());
    for name in ["_redirects", "_headers"] {
      if let Some(file) = files.iter().find(|f| f.path == name) {
        // As files, the way Wrangler sends them: as plain fields Pages ignores them.
        form = form.part(name, multipart::Part::bytes(file.bytes.clone()).file_name(name));
      }
    }

    let response = self
      .http
      .post(self.project("/deployments"))
      .bearer_auth(self.config.api_token.expose())
      .multipart(form)
      .send()
      .await
      .map_err(request_error)?;

    read(response).await
  }

  /// Sends the files Pages does not have. The project is created when it does not exist yet.
  async fn upload_assets(&self, assets: &[(&OutputFile, String)], progress: &Progress) -> Result<(), PublishError> {
    let jwt = self.upload_token().await?;
    let hashes: Vec<&str> = assets.iter().map(|(_, hash)| hash.as_str()).collect();

    let response = self
      .http
      .post(self.asset_url("/check-missing"))
      .bearer_auth(&jwt)
      .json(&json!({ "hashes": hashes }))
      .send()
      .await
      .map_err(request_error)?;
    let missing: Vec<String> = read(response).await?;
    let wanted: HashMap<&str, &OutputFile> = assets.iter().map(|(file, hash)| (hash.as_str(), *file)).collect();
    progress.start(missing.len());

    let mut bucket: Vec<Value> = Vec::new();
    let mut size = 0;
    for hash in &missing {
      let file = wanted
        .get(hash.as_str())
        .ok_or_else(|| PublishError::Other(anyhow::anyhow!("Cloudflare asked for a file that is not in the site: {hash}")))?;
      let encoded = STANDARD.encode(&file.bytes);
      size += encoded.len();
      bucket.push(json!({
        "key": hash,
        "value": encoded,
        "metadata": { "contentType": mime_guess::from_path(&file.path).first_or_octet_stream().to_string() },
        "base64": true,
      }));

      if size >= BUCKET_BYTES || bucket.len() >= BUCKET_FILES {
        let count = bucket.len();
        self.send_bucket(&jwt, std::mem::take(&mut bucket)).await?;
        progress.advance(count);
        size = 0;
      }
    }
    if !bucket.is_empty() {
      let count = bucket.len();
      self.send_bucket(&jwt, bucket).await?;
      progress.advance(count);
    }

    // Every hash of the deployment is recorded, so that what is already stored is not purged as unused.
    let response = self
      .http
      .post(self.asset_url("/upsert-hashes"))
      .bearer_auth(&jwt)
      .json(&json!({ "hashes": hashes }))
      .send()
      .await
      .map_err(request_error)?;
    read_ok(response).await
  }

  async fn send_bucket(&self, jwt: &str, bucket: Vec<Value>) -> Result<(), PublishError> {
    let response = self.http.post(self.asset_url("/upload")).bearer_auth(jwt).json(&bucket).send().await.map_err(request_error)?;

    read_ok(response).await
  }

  /// Assets are not under the account: the upload token says whose they are.
  fn asset_url(&self, path: &str) -> String {
    format!("{}/pages/assets{path}", self.config.api_url)
  }

  async fn upload_token(&self) -> Result<String, PublishError> {
    let url = self.project("/upload-token");
    let response = self.http.get(&url).bearer_auth(self.config.api_token.expose()).send().await.map_err(request_error)?;

    match read::<Token>(response).await {
      Ok(token) => Ok(token.jwt),
      // The project does not exist yet: make it, then ask again.
      Err(PublishError::Other(error)) if error.to_string().contains("answered 404") => {
        self.create_project().await?;
        let response = self.http.get(&url).bearer_auth(self.config.api_token.expose()).send().await.map_err(request_error)?;

        Ok(read::<Token>(response).await?.jwt)
      }
      Err(error) => Err(error),
    }
  }

  async fn create_project(&self) -> Result<(), PublishError> {
    let body = json!({ "name": self.config.project, "production_branch": self.config.production_branch });
    let response = self
      .http
      .post(self.project(""))
      .bearer_auth(self.config.api_token.expose())
      .json(&body)
      .send()
      .await
      .map_err(request_error)?;

    read_ok(response).await
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::{get, post},
  };

  use super::*;

  fn file(path: &str, text: &str) -> OutputFile {
    OutputFile {
      path: path.into(),
      bytes: text.as_bytes().to_vec(),
    }
  }

  #[derive(Default)]
  struct Pages {
    project_exists: bool,
    created: bool,
    known: Vec<String>,
    /// The hashes of every upload request.
    uploads: Vec<Vec<String>>,
    upserted: Vec<String>,
    /// (branch, manifest size, `_redirects`) of every deployment.
    deployments: Vec<(String, usize, Option<String>)>,
  }

  type Shared = Arc<Mutex<Pages>>;

  /// A Pages that keeps what it was sent.
  async fn start() -> (PagesTarget, Shared) {
    let state = Shared::default();
    let app = Router::new()
      .route(
        "/accounts/acct/pages/projects/docs/upload-token",
        get(|State(state): State<Shared>| async move {
          if state.lock().unwrap().project_exists {
            (StatusCode::OK, Json(json!({ "success": true, "result": { "jwt": "upload" } })))
          } else {
            (
              StatusCode::NOT_FOUND,
              Json(json!({ "success": false, "errors": [{ "code": 8000007, "message": "Project not found." }] })),
            )
          }
        }),
      )
      .route(
        "/accounts/acct/pages/projects",
        post(|State(state): State<Shared>, Json(body): Json<Value>| async move {
          assert_eq!((body["name"].as_str(), body["production_branch"].as_str()), (Some("docs"), Some("main")));
          let mut pages = state.lock().unwrap();
          pages.project_exists = true;
          pages.created = true;
          Json(json!({ "success": true, "result": { "name": "docs" } }))
        }),
      )
      .route(
        "/pages/assets/check-missing",
        post(|State(state): State<Shared>, headers: axum::http::HeaderMap, Json(body): Json<Value>| async move {
          assert_eq!(headers["authorization"], "Bearer upload", "asset calls carry the upload token");
          let pages = state.lock().unwrap();
          let missing: Vec<&Value> = body["hashes"].as_array().unwrap().iter().filter(|h| !pages.known.contains(&h.as_str().unwrap().to_string())).collect();
          Json(json!({ "success": true, "result": missing }))
        }),
      )
      .route(
        "/pages/assets/upload",
        post(|State(state): State<Shared>, Json(body): Json<Vec<Value>>| async move {
          let mut pages = state.lock().unwrap();
          let hashes: Vec<String> = body.iter().map(|f| f["key"].as_str().unwrap().to_string()).collect();
          assert!(body.iter().all(|f| f["base64"] == true && f["metadata"]["contentType"].is_string()));
          pages.known.extend(hashes.clone());
          pages.uploads.push(hashes);
          Json(json!({ "success": true, "result": { "successful_key_count": body.len() } }))
        }),
      )
      .route(
        "/pages/assets/upsert-hashes",
        post(|State(state): State<Shared>, Json(body): Json<Value>| async move {
          state.lock().unwrap().upserted = body["hashes"].as_array().unwrap().iter().map(|h| h.as_str().unwrap().to_string()).collect();
          Json(json!({ "success": true, "result": true }))
        }),
      )
      .route(
        "/accounts/acct/pages/projects/docs/deployments",
        post(|State(state): State<Shared>, mut form: Multipart| async move {
          let (mut branch, mut manifest, mut redirects) = (String::new(), 0, None);
          while let Some(field) = form.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let text = field.text().await.unwrap();
            match name.as_str() {
              "branch" => branch = text,
              "manifest" => manifest = serde_json::from_str::<HashMap<String, String>>(&text).unwrap().len(),
              "_redirects" => redirects = Some(text),
              _ => {}
            }
          }
          let mut pages = state.lock().unwrap();
          pages.deployments.push((branch.clone(), manifest, redirects));
          let id = format!("dep-{}", pages.deployments.len());
          Json(json!({ "success": true, "result": { "id": id, "url": format!("https://{id}.docs.pages.dev") } }))
        }),
      )
      .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let target = PagesTarget::new(PagesConfig {
      account_id: "acct".into(),
      api_token: "token".into(),
      project: "docs".into(),
      production_branch: "main".into(),
      preview_branch: "preview".into(),
      api_url: format!("http://{address}"),
    })
    .unwrap();

    (target, state)
  }

  #[tokio::test]
  async fn the_project_is_created_and_only_new_files_are_uploaded() {
    let (target, pages) = start().await;
    let site = |guide: &str| {
      vec![
        file("index.html", "home"),
        file("guide/index.html", guide),
        file("a.css", "body{}"),
        file("_redirects", "/old/ /new/ 301"),
      ]
    };

    target.publish(&site("v1"), &Progress::none()).await.unwrap();
    {
      let state = pages.lock().unwrap();
      assert!(state.created, "a missing project is created");
      assert_eq!(state.uploads.iter().map(Vec::len).sum::<usize>(), 3, "the `_redirects` file is a setting, not an asset");
      assert_eq!(state.upserted.len(), 3);
      assert_eq!(state.deployments, [("main".to_string(), 3, Some("/old/ /new/ 301".to_string()))]);
    }

    // One page changed: one file goes up, the manifest still names all three.
    target.publish(&site("v2"), &Progress::none()).await.unwrap();
    let state = pages.lock().unwrap();
    assert_eq!(state.uploads.last().map(Vec::len), Some(1));
    assert_eq!(state.deployments.len(), 2);
    assert_eq!(state.deployments[1].1, 3);
  }

  #[tokio::test]
  async fn a_preview_goes_to_its_own_branch_and_has_an_address() {
    let (target, pages) = start().await;
    pages.lock().unwrap().project_exists = true;

    let (id, url) = target.upload(&[file("index.html", "new")], &Progress::none()).await.unwrap();

    assert_eq!((id.as_str(), url.as_deref()), ("dep-1", Some("https://dep-1.docs.pages.dev")));
    assert_eq!(pages.lock().unwrap().deployments[0].0, "preview", "not the production branch");
    assert!(!pages.lock().unwrap().created);
  }

  #[test]
  fn pages_allows_fewer_files_than_workers() {
    let files: Vec<OutputFile> = (0..20_001).map(|i| file(&format!("p{i}.html"), "x")).collect();

    assert_eq!(PagesTarget::check(&files).len(), 1);
    assert!(PagesTarget::check(&files[..20_000]).is_empty());
  }

  /// Against a real account: `cargo test -p cairn-publish live_pages -- --ignored` with the `CLOUDFLARE_*` variables and
  /// `CLOUDFLARE_PAGES_PROJECT` set. Deploys a one-page site to the preview branch, never to production.
  #[tokio::test]
  #[ignore = "needs a Cloudflare account"]
  async fn live_pages_accepts_a_preview_deployment() {
    let var = |name| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    let target = PagesTarget::new(PagesConfig {
      account_id: var("CLOUDFLARE_ACCOUNT_ID"),
      api_token: var("CLOUDFLARE_API_TOKEN").into(),
      project: var("CLOUDFLARE_PAGES_PROJECT"),
      production_branch: "main".into(),
      preview_branch: "preview".into(),
      api_url: "https://api.cloudflare.com/client/v4".into(),
    })
    .unwrap();

    // `LIVE_SITE_DIR`: a built release folder, to see a whole site go up.
    let files = match std::env::var("LIVE_SITE_DIR") {
      Ok(dir) => crate::cloudflare::walk(std::path::Path::new(&dir), std::path::Path::new(&dir)),
      Err(_) => vec![file("index.html", "<h1>cairn smoke test</h1>"), file("404.html", "not found"), file("_redirects", "/old/ /new/ 301")],
    };
    let result = target.upload(&files, &Progress::none()).await;

    // The failure message carries Cloudflare's own answer; the address is in the dashboard when it works.
    result.unwrap();
  }
}
