use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

use axum::{
  Json, Router,
  extract::{Path, State},
  http::{HeaderValue, StatusCode, header},
  response::{Html, IntoResponse},
  routing::{get, post},
};
use serde_json::Value;
use tracing::instrument;
use yourdocs_render::Pool;

use crate::{assets::Assets, error::AppError};

#[derive(Clone)]
pub struct AppState {
  pub pool: Arc<Pool>,
  // Only held for the length of a map access, never across an await.
  pub docs: Arc<RwLock<HashMap<String, Value>>>,
}

pub fn router(state: AppState) -> Router {
  Router::new()
    .route("/api/docs/{id}", post(save_doc))
    .route("/docs/{id}", get(show_doc))
    .route("/docs/{id}/client", get(show_doc_client_only))
    .route("/assets/client.js", get(client_js))
    .with_state(state)
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
async fn show_doc(State(state): State<AppState>, Path(id): Path<String>) -> Result<Html<String>, AppError> {
  let doc = state.docs.read().map_err(|_| poisoned())?.get(&id).cloned().ok_or(AppError::NotFound)?;
  let body = state.pool.render(doc.clone()).await?;

  Ok(Html(page(&body, &doc)))
}

/// Same page without the server render, so the browser builds the whole tree itself; the parity check compares it with `show_doc`.
#[instrument(skip_all, fields(id = %id))]
async fn show_doc_client_only(State(state): State<AppState>, Path(id): Path<String>) -> Result<Html<String>, AppError> {
  let doc = state.docs.read().map_err(|_| poisoned())?.get(&id).cloned().ok_or(AppError::NotFound)?;

  Ok(Html(page("", &doc)))
}

async fn client_js() -> impl IntoResponse {
  match Assets::get("client.js") {
    Some(file) => (
      [
        (header::CONTENT_TYPE, HeaderValue::from_static("text/javascript")),
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

const STYLE: &str = "body{font-family:system-ui,sans-serif;max-width:48rem;margin:2rem auto;padding:0 1rem}\
.tabs-list{display:flex;gap:.25rem;border-bottom:1px solid #8884}\
.tabs-tab{font:inherit;padding:.5rem 1rem;border:0;background:none;cursor:pointer;border-bottom:2px solid transparent}\
.tabs-tab[aria-selected=true]{border-bottom-color:currentColor;font-weight:600}\
.tabs-panel{padding:1rem 0}";

fn page(body: &str, doc: &Value) -> String {
  // `<` is escaped so the document can never close the script element.
  let json = doc.to_string().replace('<', "\\u003c");

  format!(
    "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>Document</title><style>{STYLE}</style></head><body><div id=\"root\">{body}</div>\
<script type=\"application/json\" id=\"doc\">{json}</script><script src=\"/assets/client.js\" defer></script></body></html>"
  )
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

  fn state() -> AppState {
    let script = Assets::get("ssr.js").expect("ssr.js is missing, run `bun run build` first");
    let pool = Pool::new(std::str::from_utf8(&script.data).unwrap(), 1, 8, Duration::from_secs(5)).unwrap();

    AppState {
      pool: Arc::new(pool),
      docs: Default::default(),
    }
  }

  #[tokio::test]
  async fn saved_document_is_served_rendered_with_its_data() {
    let state = state();
    let doc: Value = serde_json::from_str(include_str!("../../../fixtures/tabs.json")).unwrap();

    let status = save_doc(State(state.clone()), Path("a".into()), Json(doc)).await.unwrap();
    assert_eq!(status, StatusCode::NO_CONTENT);

    let Html(html) = show_doc(State(state), Path("a".into())).await.unwrap();
    for expected in ["Linux", "macOS", "id=\"doc\"", "/assets/client.js"] {
      assert!(html.contains(expected), "missing {expected}");
    }
  }

  #[tokio::test]
  async fn unknown_document_and_bad_root_are_rejected() {
    let state = state();

    assert!(matches!(show_doc(State(state.clone()), Path("nope".into())).await, Err(AppError::NotFound)));
    assert!(matches!(
      save_doc(State(state), Path("a".into()), Json(serde_json::json!({ "type": "paragraph" }))).await,
      Err(AppError::Invalid(_))
    ));
  }
}
