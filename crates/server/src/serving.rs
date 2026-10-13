//! The router of `yourdocs serve`: the editor under `/_editor`, previews under `/_preview/<release>/`, and the public site at `/`.

use axum::{
  Router,
  extract::{OriginalUri, Path, State},
  http::{HeaderMap, StatusCode, Uri, header},
  response::{IntoResponse, Redirect, Response},
  routing::get,
};
use yourdocs_core::ReleaseId;
use yourdocs_publish::PublishTarget;

use crate::{
  app::{AppState, editor_app},
  error::AppError,
  site, uploads,
};

pub(crate) fn serve_router(state: AppState) -> Router {
  let router = Router::new()
    .nest_service("/_editor", editor_app(state.auth.clone()).with_state(state.clone()))
    .route("/_a/{name}", get(asset_or_site))
    .route("/_preview/{release}", get(preview_root))
    .route("/_preview/{release}/", get(preview_root))
    .route("/_preview/{release}/{*path}", get(preview));

  // Only a self-hosted site is served here; a folder target is served by whatever the deployer put in front of it.
  match state.publisher.target() {
    PublishTarget::SelfHosted(_) => router.fallback_service(site::public_router(state.publisher.target().site_dir().expect("a self-hosted target has a site folder"))),
    PublishTarget::Folder(_) | PublishTarget::Cloudflare(_) => router,
  }
  .with_state(state)
}

/// `/_a/<id>` is two things: the original of an uploaded image for a signed-in editor, who may see it before it is published, and for
/// everyone else (and for the sized variants) a file of the published site.
async fn asset_or_site(State(state): State<AppState>, headers: HeaderMap, Path(name): Path<String>, uri: Uri) -> Response {
  if state.auth.authenticate(&headers).await.is_ok() {
    match uploads::serve(State(state.clone()), Path(name)).await {
      Ok(response) => return response,
      Err(AppError::Core(yourdocs_core::CoreError::NotFound) | AppError::Store(yourdocs_store::StoreError::NotFound)) => {}
      Err(error) => return error.into_response(),
    }
  }

  match state.publisher.target().site_dir() {
    Some(dir) if matches!(state.publisher.target(), PublishTarget::SelfHosted(_)) => site::respond(&dir, &uri, "", false).await,
    _ => StatusCode::NOT_FOUND.into_response(),
  }
}

async fn preview_root(State(state): State<AppState>, headers: HeaderMap, OriginalUri(original): OriginalUri, Path(release): Path<String>) -> Response {
  if !original.path().ends_with('/') {
    // Only an ID goes into the address: the segment is decoded, and a line break in a header would panic.
    let Ok(id) = release.parse::<ReleaseId>() else { return StatusCode::NOT_FOUND.into_response() };

    return Redirect::permanent(&format!("/_preview/{id}/")).into_response();
  }

  show_release(&state, &headers, &original, &release, "/").await
}

async fn preview(State(state): State<AppState>, headers: HeaderMap, OriginalUri(original): OriginalUri, Path((release, path)): Path<(String, String)>) -> Response {
  show_release(&state, &headers, &original, &release, &format!("/{path}")).await
}

/// A release of the site, for signed-in editors only: it may not be published, and what is in it is not public.
async fn show_release(state: &AppState, headers: &HeaderMap, original: &Uri, release: &str, path: &str) -> Response {
  if state.auth.authenticate(headers).await.is_err() {
    let next = original.path_and_query().map_or("/", |p| p.as_str());

    return Redirect::to(&format!("/_editor/auth/login?next={}", percent_encoding::utf8_percent_encode(next, percent_encoding::NON_ALPHANUMERIC))).into_response();
  }

  // Parsed as an ID, so the name cannot lead out of the releases folder.
  let (Ok(id), Some(releases)) = (release.parse::<ReleaseId>(), state.publisher.target().releases_dir()) else {
    return StatusCode::NOT_FOUND.into_response();
  };
  let dir = releases.join(id.to_string());
  if !dir.is_dir() {
    return StatusCode::NOT_FOUND.into_response();
  }

  let uri = match (path.parse::<Uri>(), original.query()) {
    (Ok(uri), _) => uri,
    _ => return StatusCode::NOT_FOUND.into_response(),
  };
  let mut response = site::respond(&dir, &uri, &format!("/_preview/{id}"), false).await;
  // A preview is not for search engines, nor for shared caches.
  response.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("private, no-store"));
  response.headers_mut().insert("x-robots-tag", header::HeaderValue::from_static("noindex"));

  response
}

#[cfg(test)]
mod tests {
  use axum::{body::Body, http::Request};
  use jiff::SignedDuration;
  use tower::ServiceExt;
  use yourdocs_auth::{AccessRules, Auth, AuthSettings, Oidc, SessionSettings};
  use yourdocs_publish::FolderTarget;

  use super::*;
  use crate::app::test_state_serving;

  const RELEASE: &str = "01a135ec-ebd6-70b5-ab47-242eef382f96";

  /// A self-hosted server that requires sign-in, with one release on disk that is also the live site.
  async fn server() -> (Router, String) {
    let (mut state, dir) = test_state_serving(|dir| PublishTarget::SelfHosted(FolderTarget::new(dir.join("out")))).await;
    state.auth = Auth::oidc(
      Oidc::unreachable(),
      None,
      state.db.clone(),
      AuthSettings {
        sessions: SessionSettings {
          idle: SignedDuration::from_hours(1),
          absolute: SignedDuration::from_hours(24),
          secure: true,
        },
        access: AccessRules {
          allow_any: true,
          ..Default::default()
        },
        public_origin: "https://docs.example.com".into(),
        base_path: "/_editor".into(),
      },
    );

    let release = dir.join("out/releases").join(RELEASE);
    std::fs::create_dir_all(release.join("guide")).unwrap();
    std::fs::write(release.join("guide/index.html"), "the guide").unwrap();
    std::fs::write(release.join("404.html"), "nothing here").unwrap();
    std::os::unix::fs::symlink(std::path::Path::new("releases").join(RELEASE), dir.join("out/site")).unwrap();

    let user = yourdocs_store::upsert_user(&state.db, "https://idp.example", "alice", "alice@example.com", "Alice").await.unwrap();
    let cookie = state.auth.start_session_for_tests(user.id).await;

    (serve_router(state), cookie)
  }

  async fn get(app: &Router, path: &str, cookie: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
      request = request.header(header::COOKIE, cookie);
    }
    let response = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();

    (status, String::from_utf8_lossy(&axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap()).into_owned())
  }

  #[tokio::test]
  async fn the_public_site_is_open_and_previews_are_not() {
    let (app, cookie) = server().await;

    // (path, signed in, status, body or empty to skip): the live site answers everyone, a preview only an editor.
    for (path, signed_in, status, body) in [
      ("/guide/", false, StatusCode::OK, "the guide"),
      ("/missing/", false, StatusCode::NOT_FOUND, "nothing here"),
      (&format!("/_preview/{RELEASE}/guide/"), false, StatusCode::SEE_OTHER, ""),
      (&format!("/_preview/{RELEASE}/guide/"), true, StatusCode::OK, "the guide"),
      ("/_preview/not-a-release/guide/", true, StatusCode::NOT_FOUND, ""),
      ("/_preview/a%0d%0aSet-Cookie:x=y", true, StatusCode::NOT_FOUND, ""),
      ("/_preview/00000000-0000-0000-0000-000000000000/guide/", true, StatusCode::NOT_FOUND, ""),
      ("/api/pages", false, StatusCode::NOT_FOUND, "nothing here"),
      ("/_editor/api/pages", false, StatusCode::UNAUTHORIZED, ""),
      ("/_editor/api/pages", true, StatusCode::OK, ""),
    ] {
      let (found, text) = get(&app, path, signed_in.then_some(cookie.as_str())).await;

      assert_eq!(found, status, "{path} signed_in={signed_in}");
      assert!(body.is_empty() || text == body, "{path}: {text}");
    }
  }
}
