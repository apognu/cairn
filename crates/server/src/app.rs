use std::{
  collections::HashMap,
  sync::{Arc, LazyLock, RwLock},
};

use axum::{
  Json, Router,
  body::Body,
  extract::{Path, State},
  http::{HeaderValue, Request, StatusCode, header},
  middleware::{self, Next},
  response::{Html, IntoResponse, Response},
  routing::get,
};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use tracing::instrument;
use yourdocs_assets::LocalAssetStore;
use yourdocs_core::LocalBroadcast;
use yourdocs_publish::{Bundles, FolderTarget, PublishConfig, PublishTarget, Publisher, PublisherParts};
use yourdocs_render::Pool;
use yourdocs_store::SqlContentStore;

use crate::{assets::Assets, drafts, error::AppError, library, links, pages, publish, redirects, settings, uploads, urls};

#[derive(Clone)]
pub struct AppState {
  pub pool: Arc<Pool>,
  // Only held for the length of a map access, never across an await.
  pub docs: Arc<RwLock<HashMap<String, Value>>>,
  pub db: DatabaseConnection,
  pub content: SqlContentStore,
  pub assets: Arc<LocalAssetStore>,
  pub publisher: Arc<Publisher<SqlContentStore, LocalAssetStore, LocalBroadcast>>,
}

/// The publisher for a data directory: the pages become a site under `<data_dir>/site`.
pub(crate) fn publisher(
  db: DatabaseConnection,
  content: SqlContentStore,
  assets: Arc<LocalAssetStore>,
  pool: Arc<Pool>,
  data_dir: &std::path::Path,
  config: PublishConfig,
  broadcast: LocalBroadcast,
) -> anyhow::Result<Arc<Publisher<SqlContentStore, LocalAssetStore, LocalBroadcast>>> {
  let embedded = |name: &str| {
    Assets::get(name)
      .map(|f| f.data.into_owned())
      .ok_or_else(|| anyhow::anyhow!("{name} is not embedded, run `bun run build` first"))
  };
  let bundles = Bundles {
    client_js: embedded("client.js")?,
    site_css: embedded("site.css")?,
  };

  Ok(Arc::new(Publisher::new(PublisherParts {
    db,
    content,
    assets,
    pool,
    target: PublishTarget::Folder(FolderTarget::new(data_dir.to_path_buf())),
    bundles,
    config,
    broadcast,
  })))
}

/// The OpenAPI document and the router serving the same routes, built from one definition.
pub fn api() -> (Router<AppState>, utoipa::openapi::OpenApi) {
  let (router, mut spec) = pages::routes()
    .merge(drafts::routes())
    .merge(uploads::routes())
    .merge(publish::routes())
    .merge(settings::routes())
    .merge(urls::routes())
    .merge(redirects::routes())
    .merge(links::routes())
    .merge(library::routes())
    .split_for_parts();
  // The router's default info describes utoipa-axum itself.
  spec.info = utoipa::openapi::InfoBuilder::new().title("yourdocs").version(env!("CARGO_PKG_VERSION")).build();

  (router, spec)
}

pub fn router(state: AppState) -> Router {
  Router::new()
    .route("/api/docs/{id}", get(load_doc).post(save_doc))
    .route("/docs/{id}", get(show_doc))
    .route("/docs/{id}/client", get(show_doc_client_only))
    .route("/assets/client.js", get(client_js))
    .route("/assets/site.css", get(site_css))
    .route("/assets/code.css", get(code_css))
    .route("/_a/{name}", get(uploads::serve))
    .merge(api().0)
    .fallback(crate::editor::serve)
    .layer(middleware::from_fn(local_host_only))
    .with_state(state)
}

/// Without a login, a page on another origin could drive the API through a DNS-rebinding name that resolves to 127.0.0.1; refusing foreign Host headers closes that.
async fn local_host_only(request: Request<Body>, next: Next) -> Response {
  let host = request.headers().get(header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("");
  let name = host.rsplit_once(':').filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit())).map_or(host, |(name, _)| name);

  if matches!(name, "localhost" | "127.0.0.1" | "[::1]") {
    next.run(request).await
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

#[instrument(skip_all, fields(id = %id))]
async fn save_doc(State(state): State<AppState>, Path(id): Path<String>, Json(doc): Json<Value>) -> Result<StatusCode, AppError> {
  if doc.get("type").and_then(Value::as_str) != Some("doc") {
    return Err(AppError::Invalid(vec!["the root node must have type \"doc\"".into()]));
  }

  state.docs.write().map_err(|_| poisoned())?.insert(id, doc);

  Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip_all, fields(id = %id))]
async fn load_doc(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
  state.docs.read().map_err(|_| poisoned())?.get(&id).cloned().map(Json).ok_or(AppError::NotFound)
}

#[instrument(skip_all, fields(id = %id))]
async fn show_doc(State(state): State<AppState>, Path(id): Path<String>) -> Result<Html<String>, AppError> {
  let doc = state.docs.read().map_err(|_| poisoned())?.get(&id).cloned().ok_or(AppError::NotFound)?;
  let body = state.pool.render(input(&doc)).await?;

  Ok(Html(page(&body, &doc)))
}

/// Same page without the server render, so the browser builds the whole tree itself; the parity check compares it with `show_doc`.
#[instrument(skip_all, fields(id = %id))]
async fn show_doc_client_only(State(state): State<AppState>, Path(id): Path<String>) -> Result<Html<String>, AppError> {
  let doc = state.docs.read().map_err(|_| poisoned())?.get(&id).cloned().ok_or(AppError::NotFound)?;

  Ok(Html(page("", &doc)))
}

async fn client_js() -> Response {
  embedded("client.js", "text/javascript")
}

async fn code_css() -> Response {
  static CSS: LazyLock<String> = LazyLock::new(yourdocs_render::highlight_css);

  ([(header::CONTENT_TYPE, "text/css"), (header::CACHE_CONTROL, "no-cache")], CSS.as_str()).into_response()
}

async fn site_css() -> Response {
  embedded("site.css", "text/css")
}

fn embedded(name: &str, content_type: &'static str) -> Response {
  match Assets::get(name) {
    Some(file) => (
      [
        (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
        (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
      ],
      file.data.into_owned(),
    )
      .into_response(),
    None => StatusCode::NOT_FOUND.into_response(),
  }
}

fn poisoned() -> AppError {
  AppError::Render(yourdocs_render::RenderError::Other(anyhow::anyhow!("document store lock poisoned")))
}

/// What `ssr.js` and the hydrating client both receive: the same input, or React would see a mismatch.
fn input(doc: &Value) -> Value {
  json!({ "doc": doc, "context": { "today": jiff::Zoned::now().date().to_string() } })
}

fn page(body: &str, doc: &Value) -> String {
  // `<` is escaped so the document can never close the script element.
  let json = input(doc).to_string().replace('<', "\\u003c");

  format!(
    "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>Document</title><link rel=\"stylesheet\" href=\"/assets/site.css\"><link rel=\"stylesheet\" href=\"/assets/code.css\"></head><body><div id=\"root\">{body}</div>\
<script type=\"application/json\" id=\"doc\">{json}</script><script src=\"/assets/client.js\" defer></script></body></html>"
  )
}

#[cfg(test)]
pub(crate) async fn test_state() -> AppState {
  test_state_with_dir().await.0
}

#[cfg(test)]
pub(crate) async fn test_state_with_dir() -> (AppState, std::path::PathBuf) {
  let script = Assets::get("ssr.js").expect("ssr.js is missing, run `bun run build` first");
  let pool = Pool::new(std::str::from_utf8(&script.data).unwrap(), 1, 8, std::time::Duration::from_secs(5)).unwrap();
  let dir = std::env::temp_dir().join(format!("yourdocs-server-{}", uuid::Uuid::now_v7()));
  std::fs::create_dir_all(&dir).unwrap();
  let db = yourdocs_store::connect(&dir.join("test.db")).await.unwrap();

  let pool = Arc::new(pool);
  let content = SqlContentStore::new(db.clone());
  let assets = Arc::new(LocalAssetStore::new(&dir.join("assets")).unwrap());

  let state = AppState {
    publisher: publisher(
      db.clone(),
      content.clone(),
      assets.clone(),
      pool.clone(),
      &dir.join("out"),
      PublishConfig {
        site_url: "http://localhost:3001".into(),
        pagefind: "pagefind-not-installed".into(),
      },
      LocalBroadcast::new(),
    )
    .unwrap(),
    pool,
    docs: Default::default(),
    content,
    assets,
    db,
  };

  (state, dir)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn saved_document_is_served_rendered_with_its_data() {
    let state = test_state().await;
    let doc: Value = serde_json::from_str(include_str!("../../../fixtures/tabs.json")).unwrap();

    let status = save_doc(State(state.clone()), Path("a".into()), Json(doc)).await.unwrap();
    assert_eq!(status, StatusCode::NO_CONTENT);

    let Html(html) = show_doc(State(state), Path("a".into())).await.unwrap();
    for expected in ["Linux", "macOS", "id=\"doc\"", "/assets/client.js"] {
      assert!(html.contains(expected), "missing {expected}");
    }
  }

  #[tokio::test]
  async fn every_fixture_renders() {
    let state = test_state().await;
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

    for entry in std::fs::read_dir(dir).unwrap() {
      let path = entry.unwrap().path();
      let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

      let html = state
        .pool
        .render(json!({ "doc": doc, "context": { "page": { "title": "Fixture" } } }))
        .await
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));

      assert!(html.contains("class=\"doc-content\""), "{}", path.display());
    }
  }

  #[tokio::test]
  async fn new_tags_disappear_after_their_date() {
    let state = test_state().await;
    let doc: Value = serde_json::from_str(include_str!("../../../fixtures/features.json")).unwrap();
    let render = |today: &'static str| {
      let (state, doc) = (state.clone(), doc.clone());
      async move { state.pool.render(json!({ "doc": doc, "context": { "today": today } })).await.unwrap() }
    };

    // The fixture has one tag dated 2099 and one dated 2000 in each component.
    assert_eq!(render("2026-10-12").await.matches("class=\"feature-new\"").count(), 2);
    assert_eq!(render("2099-12-31").await.matches("class=\"feature-new\"").count(), 2);
    assert_eq!(render("2100-01-01").await.matches("class=\"feature-new\"").count(), 0);
  }

  #[tokio::test]
  async fn unknown_document_and_bad_root_are_rejected() {
    let state = test_state().await;

    assert!(matches!(show_doc(State(state.clone()), Path("nope".into())).await, Err(AppError::NotFound)));
    assert!(matches!(
      save_doc(State(state), Path("a".into()), Json(serde_json::json!({ "type": "paragraph" }))).await,
      Err(AppError::Invalid(_))
    ));
  }
}
