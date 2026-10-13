use std::sync::Arc;

use axum::{
  Router,
  body::Body,
  http::{Method, Request, StatusCode, header},
  middleware::{self, Next},
  response::{IntoResponse, Response},
  routing::get,
};
use sea_orm::DatabaseConnection;
use yourdocs_assets::LocalAssetStore;
use yourdocs_core::LocalBroadcast;
use yourdocs_publish::{Bundles, FolderTarget, PublishConfig, PublishTarget, Publisher, PublisherParts};
use yourdocs_render::Pool;
use yourdocs_store::SqlContentStore;

use crate::{drafts, history, library, links, pages, publish, redirects, settings, uploads, urls};

#[derive(Clone)]
pub struct AppState {
  pub pool: Arc<Pool>,
  pub db: DatabaseConnection,
  pub content: SqlContentStore,
  pub assets: Arc<LocalAssetStore>,
  pub publisher: Arc<Publisher<SqlContentStore, LocalAssetStore, LocalBroadcast>>,
  /// The editor built with the custom components, when there is one; otherwise the embedded editor is served.
  pub editor_dir: Option<Arc<std::path::PathBuf>>,
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
  assets: Arc<LocalAssetStore>,
  pool: Arc<Pool>,
  data_dir: &std::path::Path,
  setup: PublishSetup,
  broadcast: LocalBroadcast,
) -> anyhow::Result<Arc<Publisher<SqlContentStore, LocalAssetStore, LocalBroadcast>>> {
  Ok(Arc::new(Publisher::new(PublisherParts {
    db,
    content,
    assets,
    pool,
    target: PublishTarget::Folder(FolderTarget::new(data_dir.to_path_buf())),
    bundles: setup.bundles,
    config: setup.config,
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
    .merge(history::routes())
    .split_for_parts();
  // The router's default info describes utoipa-axum itself.
  spec.info = utoipa::openapi::InfoBuilder::new().title("yourdocs").version(env!("CARGO_PKG_VERSION")).build();

  (router, spec)
}

pub fn router(state: AppState) -> Router {
  Router::new()
    .route("/_a/{name}", get(uploads::serve))
    .merge(api().0)
    .fallback(crate::editor::serve)
    .layer(middleware::from_fn(local_host_only))
    .with_state(state)
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
  let bundles = crate::bundles::BundleSet::load(None, false).expect("the bundles are missing, run `bun run build` first");
  let pool = Pool::new(&bundles.ssr_js, 1, 8, std::time::Duration::from_secs(5)).unwrap();
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
      PublishSetup {
        bundles: Bundles::new(&bundles.ssr_js, bundles.client_js, bundles.site_css),
        config: PublishConfig {
          site_url: "http://localhost:3001".into(),
          pagefind: "pagefind-not-installed".into(),
        },
      },
      LocalBroadcast::new(),
    )
    .unwrap(),
    pool,
    content,
    assets,
    db,
    editor_dir: None,
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
