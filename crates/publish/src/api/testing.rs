//! A stand-in for GitHub that tests can change between publishes: latest releases, files and assets, and a counter of requests.

use std::{
  collections::HashMap,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
};

use axum::{
  Router,
  extract::State,
  http::{StatusCode, Uri, header},
  response::{IntoResponse, Response},
};

use super::ApiHosts;

#[derive(Default)]
struct Inner {
  latest: Mutex<HashMap<String, String>>,
  files: Mutex<HashMap<String, String>>,
  hits: AtomicUsize,
}

#[derive(Clone)]
pub(crate) struct StandIn {
  base: String,
  inner: Arc<Inner>,
}

impl StandIn {
  pub(crate) async fn start() -> StandIn {
    let inner = Arc::new(Inner::default());
    let app = Router::new().fallback(handle).with_state(inner.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    StandIn { base, inner }
  }

  pub(crate) fn hosts(&self) -> ApiHosts {
    ApiHosts {
      github: self.base.clone(),
      raw: self.base.clone(),
    }
  }

  /// `owner/name` has this tag as its latest release.
  pub(crate) fn set_latest(&self, repo: &str, tag: &str) {
    self.inner.latest.lock().unwrap().insert(repo.into(), tag.into());
  }

  /// Serves `body` at `path` (`/acme/orders/releases/download/v1/openapi.json`, `/acme/orders/main/spec.yaml`).
  pub(crate) fn set_file(&self, path: &str, body: &str) {
    self.inner.files.lock().unwrap().insert(path.into(), body.into());
  }

  pub(crate) fn remove_file(&self, path: &str) {
    self.inner.files.lock().unwrap().remove(path);
  }

  pub(crate) fn hits(&self) -> usize {
    self.inner.hits.load(Ordering::Relaxed)
  }
}

async fn handle(State(inner): State<Arc<Inner>>, uri: Uri) -> Response {
  inner.hits.fetch_add(1, Ordering::Relaxed);
  let path = uri.path();

  if let Some(repo) = path.strip_suffix("/releases/latest") {
    let tag = inner.latest.lock().unwrap().get(repo.trim_start_matches('/')).cloned();

    return match tag {
      Some(tag) => (StatusCode::FOUND, [(header::LOCATION, format!("{repo}/releases/tag/{tag}"))]).into_response(),
      None => StatusCode::NOT_FOUND.into_response(),
    };
  }

  let body = inner.files.lock().unwrap().get(path).cloned();
  match body {
    Some(body) => body.into_response(),
    None => StatusCode::NOT_FOUND.into_response(),
  }
}

/// A small OpenAPI document with one operation.
pub(crate) fn spec(title: &str, version: &str, method: &str, path: &str, operation_id: &str) -> String {
  serde_json::json!({
    "openapi": "3.0.3",
    "info": { "title": title, "version": version },
    "paths": { path: { method: { "operationId": operation_id, "summary": format!("{operation_id} summary"), "responses": { "200": { "description": "ok" } } } } },
  })
  .to_string()
}
