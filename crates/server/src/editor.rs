use axum::{
  extract::{OriginalUri, State},
  http::{StatusCode, Uri, header},
  response::{IntoResponse, Redirect, Response},
};
use rust_embed::Embed;

use crate::app::AppState;

/// The editor SPA built by `bun run build` in `packages/editor`.
#[derive(Embed)]
#[folder = "../../packages/editor/dist"]
struct Editor;

/// A file of the editor and its content type: from the directory `build-components` made when the site has custom components, else from the binary.
async fn file(state: &AppState, path: &str) -> Option<(String, Vec<u8>)> {
  let Some(dir) = &state.editor_dir else {
    return Editor::get(path).map(|f| (f.metadata.mimetype().to_string(), f.data.into_owned()));
  };

  // The path comes from the URL: nothing outside the editor's folder may be read.
  if path.split('/').any(|part| part == ".." || part.is_empty() || part.contains('\\')) {
    return None;
  }

  let bytes = tokio::fs::read(dir.join(path)).await.ok()?;

  Some((mime_guess::from_path(path).first_or_octet_stream().to_string(), bytes))
}

/// Serves the editor's files, and its entry page for any other path so the SPA can own client-side URLs.
pub(crate) async fn serve(State(state): State<AppState>, OriginalUri(original): OriginalUri, uri: Uri) -> Response {
  let path = uri.path().trim_start_matches('/');

  // The editor's files are linked relative to its address, which must therefore end in a slash.
  if original.path().ends_with("/_editor") {
    return Redirect::permanent("/_editor/").into_response();
  }

  // Unknown API paths must stay 404s rather than turn into HTML.
  if path.starts_with("api/") {
    return StatusCode::NOT_FOUND.into_response();
  }

  if let Some((mime, bytes)) = file(&state, path).await {
    // Vite fingerprints everything under assets/, so those files never change.
    let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };

    return ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache.to_string())], bytes).into_response();
  }

  // A missing file is a 404, not the SPA: a script that comes back as HTML fails with a confusing syntax error.
  if path.rsplit('/').next().is_some_and(|name| name.contains('.')) {
    return StatusCode::NOT_FOUND.into_response();
  }

  match file(&state, "index.html").await {
    Some((_, index)) => ([(header::CONTENT_TYPE, "text/html; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")], index).into_response(),
    None => (StatusCode::NOT_FOUND, "the editor is not embedded, run `bun run build` before `cargo build`").into_response(),
  }
}
