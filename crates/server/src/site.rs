//! The published site, served from the folder a publish wrote, with its redirect rules and 404 page applied. `dev` adds a script that
//! reloads open tabs when a new release goes live; `serve` serves the same folder at the site's address, and a release under `/_preview/`.

use std::{
  convert::Infallible,
  path::{Component, Path, PathBuf},
};

use axum::{
  Router,
  extract::State,
  http::{HeaderValue, StatusCode, Uri, header},
  response::{IntoResponse, Response, Sse, sse::Event as SseEvent},
  routing::get,
};
use cairn_core::{Broadcast, Event, LocalBroadcast};
use futures_util::{Stream, StreamExt};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
struct SiteState {
  /// The `site` link a publish switches; resolved on every request so a new release is served at once.
  dir: PathBuf,
  /// Set in `dev`, which reloads open tabs after a publish.
  events: Option<LocalBroadcast>,
}

/// The site of `dir` for `dev`, with the reload script.
pub(crate) fn dev_router(dir: PathBuf, events: LocalBroadcast) -> Router {
  Router::new().route("/__reload", get(reload)).fallback(serve).with_state(SiteState { dir, events: Some(events) })
}

/// The site of `dir` as readers get it.
pub(crate) fn public_router(dir: PathBuf) -> Router {
  Router::new().fallback(serve).with_state(SiteState { dir, events: None })
}

/// Server-sent events: one `reload` message per published release.
async fn reload(State(state): State<SiteState>) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
  let events = state.events.unwrap_or_default();
  let stream = BroadcastStream::new(events.subscribe()).filter_map(|event| async move { matches!(event, Ok(Event::Published { .. })).then(|| Ok(SseEvent::default().data("reload"))) });

  Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

const RELOAD_SCRIPT: &str = "<script>new EventSource('/__reload').onmessage=()=>location.reload()</script>";

async fn serve(State(state): State<SiteState>, uri: Uri) -> Response {
  respond(&state.dir, &uri, "", state.events.is_some()).await
}

/// The answer to a request for `uri` (relative to the site root) from the site in `dir`. `prefix` is where the site is mounted when that
/// is not the root, so that the redirects it sends stay inside it; `reload` adds the script `dev` uses.
pub(crate) async fn respond(dir: &Path, uri: &Uri, prefix: &str, reload: bool) -> Response {
  let state = (dir, reload);
  let path = percent_encoding::percent_decode_str(uri.path()).decode_utf8_lossy().into_owned();

  if let Some((to, status)) = redirect_for(dir, &path).await {
    let to = if to.starts_with('/') && !to.starts_with("//") { format!("{prefix}{to}") } else { to };

    return (status, [(header::LOCATION, to)]).into_response();
  }

  let Some(relative) = safe_relative(&path) else { return not_found(state).await };
  let candidate = dir.join(&relative);

  // `/a/b/` is the page `a/b/index.html`; `/a/b` without the slash goes to the canonical address.
  let file = if path.ends_with('/') || relative.as_os_str().is_empty() {
    candidate.join("index.html")
  } else if candidate.is_file() {
    candidate
  } else if candidate.join("index.html").is_file() {
    let query = uri.query().map(|q| format!("?{q}")).unwrap_or_default();
    // One leading slash: `//guide/` would be read as the host `guide`. The raw path keeps its encoding, as a header needs.
    let location = format!("{prefix}/{}/{query}", uri.path().trim_start_matches('/'));
    return (StatusCode::MOVED_PERMANENTLY, [(header::LOCATION, location)]).into_response();
  } else {
    return not_found(state).await;
  };

  match tokio::fs::read(&file).await {
    Ok(bytes) => file_response(&file, &relative, bytes, StatusCode::OK, reload),
    Err(_) => not_found(state).await,
  }
}

/// A request path as a relative file path, or `None` if it tries to leave the site folder.
fn safe_relative(path: &str) -> Option<PathBuf> {
  let relative = Path::new(path.trim_start_matches('/'));

  relative
    .components()
    .all(|c| matches!(c, Component::Normal(_)))
    .then(|| relative.to_path_buf())
    .or_else(|| path.trim_matches('/').is_empty().then(PathBuf::new))
}

/// The `_redirects` rule for this path, if any: lines of `from to [status]`, matched exactly.
async fn redirect_for(dir: &Path, path: &str) -> Option<(String, StatusCode)> {
  let rules = tokio::fs::read_to_string(dir.join("_redirects")).await.ok()?;

  rules.lines().find_map(|line| {
    let mut parts = line.split_whitespace();
    let (from, to) = (parts.next()?, parts.next()?);
    let status = parts.next().and_then(|s| s.parse().ok()).and_then(|s| StatusCode::from_u16(s).ok()).unwrap_or(StatusCode::FOUND);

    (from == path).then(|| (to.to_string(), status))
  })
}

async fn not_found((dir, reload): (&Path, bool)) -> Response {
  match tokio::fs::read(dir.join("404.html")).await {
    Ok(bytes) => file_response(Path::new("404.html"), Path::new("404.html"), bytes, StatusCode::NOT_FOUND, reload),
    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
  }
}

fn file_response(file: &Path, relative: &Path, bytes: Vec<u8>, status: StatusCode, reload: bool) -> Response {
  let mime = mime_guess::from_path(file).first_or_octet_stream();
  let is_html = reload && mime.essence_str() == "text/html";

  let body = if is_html {
    let html = String::from_utf8_lossy(&bytes).replacen("</body>", &format!("{RELOAD_SCRIPT}</body>"), 1);
    html.into_bytes()
  } else {
    bytes
  };

  // Hashed bundles and uploaded images never change under their name.
  let immutable = relative.starts_with("assets") || relative.starts_with("_a");
  let cache = if immutable { "public, max-age=31536000, immutable" } else { "no-cache" };

  (
    status,
    [
      (
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("application/octet-stream")),
      ),
      (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
    ],
    body,
  )
    .into_response()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn request_paths_cannot_leave_the_site_folder() {
    for (path, expected) in [("/", Some("")), ("/a/b/", Some("a/b/")), ("/a/../../etc/passwd", None), ("/..", None)] {
      assert_eq!(safe_relative(path), expected.map(PathBuf::from), "{path}");
    }
  }

  #[tokio::test]
  async fn the_slash_redirect_stays_on_the_site() {
    let dir = std::env::temp_dir().join(format!("cairn-site-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(dir.join("guide")).unwrap();
    std::fs::write(dir.join("guide/index.html"), "guide").unwrap();

    // (prefix, path, expected): under a mount point the redirect stays inside it.
    for (prefix, path, expected) in [
      ("", "/guide", "/guide/"),
      ("", "//guide", "/guide/"),
      ("", "/guide?a=1", "/guide/?a=1"),
      ("/_preview/r1", "/guide", "/_preview/r1/guide/"),
    ] {
      let response = respond(&dir, &path.parse().unwrap(), prefix, false).await;
      assert_eq!(response.headers()[header::LOCATION], expected, "{path}");
    }
  }
}
