use axum::{
  http::{StatusCode, Uri, header},
  response::{IntoResponse, Response},
};
use rust_embed::Embed;

/// The editor SPA built by `bun run build` in `packages/editor`.
#[derive(Embed)]
#[folder = "../../packages/editor/dist"]
struct Editor;

/// Serves the editor's files, and its entry page for any other path so the SPA can own client-side URLs.
pub(crate) async fn serve(uri: Uri) -> Response {
  let path = uri.path().trim_start_matches('/');

  // Unknown API paths must stay 404s rather than turn into HTML.
  if path.starts_with("api/") {
    return StatusCode::NOT_FOUND.into_response();
  }

  if let Some(file) = Editor::get(path) {
    // Vite fingerprints everything under assets/, so those files never change.
    let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };

    return (
      [(header::CONTENT_TYPE, file.metadata.mimetype().to_string()), (header::CACHE_CONTROL, cache.to_string())],
      file.data.into_owned(),
    )
      .into_response();
  }

  match Editor::get("index.html") {
    Some(index) => ([(header::CONTENT_TYPE, "text/html; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")], index.data.into_owned()).into_response(),
    None => (StatusCode::NOT_FOUND, "the editor is not embedded, run `bun run build` before `cargo build`").into_response(),
  }
}
