use std::sync::Arc;

use axum::{
  Router,
  body::Body,
  http::{Method, Request, StatusCode, header},
  middleware::{self, Next},
  response::{IntoResponse, Response},
  routing::get,
};
use cairn_assets::ObjectAssetStore;
use cairn_core::LocalBroadcast;
use cairn_publish::{Bundles, PublishConfig, PublishTarget, Publisher, PublisherParts};
use cairn_render::Pool;
use cairn_store::SqlContentStore;
use sea_orm::DatabaseConnection;

use crate::{api_pages, drafts, highlight, history, jobs, library, links, locks, me, pages, publish, redirects, settings, tokens, uploads, urls};

#[derive(Clone)]
pub struct AppState {
  pub auth: cairn_auth::Auth,
  pub pool: Arc<Pool>,
  pub db: DatabaseConnection,
  pub content: SqlContentStore,
  pub assets: Arc<ObjectAssetStore>,
  pub publisher: Arc<Publisher<SqlContentStore, ObjectAssetStore, LocalBroadcast>>,
  /// The editor built with the custom components, when there is one; otherwise the embedded editor is served.
  pub editor_dir: Option<Arc<std::path::PathBuf>>,
  /// Which API pages are being checked right now, and the spec each check last merged.
  pub api_checks: Arc<crate::api_pages::ApiChecks>,
  /// The API reference script, which the editor's preview loads from `/api/scalar.js`.
  pub scalar_js: Arc<Vec<u8>>,
  /// Turns true when the server is asked to stop.
  pub stopping: tokio::sync::watch::Receiver<bool>,
  /// How long unused uploads are kept before the jobs endpoint deletes them.
  pub asset_grace: jiff::SignedDuration,
}

impl AppState {
  /// Resolves when the server is stopping. Event streams end with it: a graceful shutdown waits for every open request, and an editor's
  /// stream would otherwise keep the server up until the tab closes.
  pub(crate) fn until_stopping(&self) -> impl Future<Output = ()> + Send + use<> {
    let mut stopping = self.stopping.clone();
    async move {
      // A dropped sender (tests, or `dev`, which has no graceful shutdown) never stops the streams.
      if stopping.wait_for(|stop| *stop).await.is_err() {
        std::future::pending::<()>().await;
      }
    }
  }
}

impl axum::extract::FromRef<AppState> for cairn_auth::Auth {
  fn from_ref(state: &AppState) -> Self {
    state.auth.clone()
  }
}

/// What the publisher needs besides the stores: the browser files every page links to, and the deployment's settings.
pub(crate) struct PublishSetup {
  pub(crate) bundles: Bundles,
  pub(crate) config: PublishConfig,
}

/// The publisher for a data directory: the pages become a site under `<data_dir>/site`.
pub(crate) fn publisher(
  db: DatabaseConnection,
  content: SqlContentStore,
  assets: Arc<ObjectAssetStore>,
  pool: Arc<Pool>,
  setup: PublishSetup,
  broadcast: LocalBroadcast,
  target: PublishTarget,
) -> anyhow::Result<Arc<Publisher<SqlContentStore, ObjectAssetStore, LocalBroadcast>>> {
  Ok(Arc::new(Publisher::new(PublisherParts {
    db,
    content,
    assets,
    pool,
    target,
    bundles: setup.bundles,
    config: setup.config,
    broadcast,
  })))
}

/// The OpenAPI document and the router serving the same routes, built from one definition.
pub fn api() -> (Router<AppState>, utoipa::openapi::OpenApi) {
  let (router, mut spec) = pages::routes()
    .merge(api_pages::routes())
    .merge(drafts::routes())
    .merge(locks::routes())
    .merge(uploads::routes())
    .merge(publish::routes())
    .merge(settings::routes())
    .merge(urls::routes())
    .merge(redirects::routes())
    .merge(links::routes())
    .merge(highlight::routes())
    .merge(library::routes())
    .merge(history::routes())
    .merge(me::routes())
    .merge(tokens::routes())
    .merge(jobs::routes())
    .split_for_parts();
  // The router's default info describes utoipa-axum itself.
  spec.info = utoipa::openapi::InfoBuilder::new().title("cairn").version(env!("CARGO_PKG_VERSION")).build();

  (router, spec)
}

/// The editor's routes: the API, the sign-in, and the editor itself as the fallback. They are not tied to a path prefix; `serve` nests
/// them under `/_editor`, `dev` serves them at the root.
pub(crate) fn editor_app(auth: cairn_auth::Auth) -> Router<AppState> {
  let auth_routes = cairn_auth::Auth::routes().with_state(auth.clone());
  // Everything that reads or changes the site's content needs a login; only the sign-in routes and the editor's own files are open.
  let guard = middleware::from_fn_with_state(auth, cairn_auth::require_login);
  let content = Router::new().route("/_a/{name}", get(uploads::serve)).merge(api().0).layer(guard);

  Router::new()
    .merge(auth_routes)
    .merge(content)
    .fallback(crate::editor::serve)
    .layer(middleware::from_fn(crate::error::rejections_as_json))
}

/// The router `dev` serves: with no login, it also refuses requests that did not come from this machine's own editor.
pub fn dev_router(state: AppState) -> Router {
  editor_app(state.auth.clone()).layer(middleware::from_fn(local_host_only)).with_state(state)
}

/// Without a login, a page on another origin could drive the API through a DNS-rebinding name that resolves to 127.0.0.1; refusing foreign Host headers closes that.
/// It could also post a plain form or a `no-cors` fetch to `localhost` directly, which needs no preflight: changes must come from the editor's own origin.
async fn local_host_only(request: Request<Body>, next: Next) -> Response {
  let headers = request.headers();
  let host = headers.get(header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("");
  let name = host.rsplit_once(':').filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit())).map_or(host, |(name, _)| name);

  if !matches!(name, "localhost" | "127.0.0.1" | "[::1]") {
    return StatusCode::FORBIDDEN.into_response();
  }

  // Browsers send Origin with every cross-origin request that changes something; tools such as curl send none and are local anyway.
  let changes = !matches!(*request.method(), Method::GET | Method::HEAD | Method::OPTIONS);
  let origin = headers.get(header::ORIGIN).map(|o| o.to_str().unwrap_or(""));
  if changes
    && let Some(origin) = origin
    && origin.split_once("://").map(|(_, authority)| authority) != Some(host)
  {
    return StatusCode::FORBIDDEN.into_response();
  }

  next.run(request).await
}

#[cfg(test)]
pub(crate) async fn test_state() -> AppState {
  test_state_with_dir().await.0
}

#[cfg(test)]
pub(crate) async fn test_state_with_dir() -> (AppState, std::path::PathBuf) {
  test_state_serving(|dir| PublishTarget::Folder(cairn_publish::FolderTarget::new(dir.join("out")))).await
}

/// A state whose publisher writes where `target` says, which is given the test's directory.
#[cfg(test)]
pub(crate) async fn test_state_serving(target: impl FnOnce(&std::path::Path) -> PublishTarget) -> (AppState, std::path::PathBuf) {
  let bundles = crate::bundles::BundleSet::load(None, false).expect("the bundles are missing, run `bun run build` first");
  let pool = Pool::new(&bundles.ssr_js, 1, 8, std::time::Duration::from_secs(5)).unwrap();
  let dir = std::env::temp_dir().join(format!("cairn-server-{}", uuid::Uuid::now_v7()));
  std::fs::create_dir_all(&dir).unwrap();
  let db = cairn_store::connect_for_tests(&dir).await;

  let pool = Arc::new(pool);
  let content = SqlContentStore::new(db.clone());
  let assets = Arc::new(ObjectAssetStore::local(&dir.join("assets")).unwrap());

  let state = AppState {
    publisher: publisher(
      db.clone(),
      content.clone(),
      assets.clone(),
      pool.clone(),
      PublishSetup {
        bundles: Bundles::new(&bundles.ssr_js, bundles.client_js, bundles.site_css, bundles.scalar_js),
        config: PublishConfig {
          site_url: "http://localhost:3001".into(),
          pagefind: "pagefind-not-installed".into(),
          api_hosts: Default::default(),
        },
      },
      LocalBroadcast::new(),
      target(&dir),
    )
    .unwrap(),
    stopping: tokio::sync::watch::channel(false).1,
    asset_grace: crate::jobs::DEFAULT_ASSET_GRACE,
    pool,
    content,
    assets,
    db,
    auth: cairn_auth::Auth::local(),
    editor_dir: None,
    api_checks: Default::default(),
    scalar_js: Arc::new(Vec::new()),
  };

  (state, dir)
}

#[cfg(test)]
mod tests {
  use axum::routing::post;
  use tower::ServiceExt;

  use super::*;

  #[tokio::test]
  async fn changes_are_accepted_only_from_the_editor_origin() {
    let app = Router::new().route("/api/publish", post(|| async { "ok" })).layer(middleware::from_fn(local_host_only));

    // (Host, Origin, expected status): no Origin is a local tool; a foreign Origin is another site posting a form.
    for (host, origin, expected) in [
      ("127.0.0.1:3000", None, StatusCode::OK),
      ("127.0.0.1:3000", Some("http://127.0.0.1:3000"), StatusCode::OK),
      ("localhost:5173", Some("http://localhost:5173"), StatusCode::OK),
      ("127.0.0.1:3000", Some("https://evil.example"), StatusCode::FORBIDDEN),
      ("127.0.0.1:3000", Some("http://localhost:8080"), StatusCode::FORBIDDEN),
      ("127.0.0.1:3000", Some("null"), StatusCode::FORBIDDEN),
      ("evil.example", None, StatusCode::FORBIDDEN),
    ] {
      let mut request = Request::post("/api/publish").header(header::HOST, host);
      if let Some(origin) = origin {
        request = request.header(header::ORIGIN, origin);
      }

      let response = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
      assert_eq!(response.status(), expected, "{host} {origin:?}");
    }
  }
}
