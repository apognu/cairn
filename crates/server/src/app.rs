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

use crate::{api_pages, drafts, highlight, history, jobs, library, links, locks, me, pages, publish, redirects, reviews, settings, tokens, uploads, urls, users};

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
    .merge(reviews::routes())
    .merge(users::routes())
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
  use cairn_core::Role;
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

  /// The least role each operation of the API needs. Every operation of the OpenAPI spec must be listed, so a new endpoint cannot be added
  /// without deciding who may call it.
  const MINIMUM_ROLES: &[(&str, &str, Role)] = &[
    ("GET", "/api/events", Role::Viewer),
    ("POST", "/api/highlight", Role::Viewer),
    ("GET", "/api/library", Role::Viewer),
    ("DELETE", "/api/library/{id}", Role::Editor),
    ("PUT", "/api/library/snippets/{key}", Role::Editor),
    ("PUT", "/api/library/terms", Role::Editor),
    ("PUT", "/api/library/variables/{key}", Role::Editor),
    ("GET", "/api/locks", Role::Viewer),
    ("GET", "/api/me", Role::Viewer),
    ("GET", "/api/pages", Role::Viewer),
    ("POST", "/api/pages", Role::Editor),
    ("POST", "/api/pages/{id}", Role::Editor),
    ("DELETE", "/api/pages/{id}", Role::Editor),
    ("POST", "/api/pages/{id}/aliases", Role::Editor),
    ("DELETE", "/api/pages/{id}/aliases", Role::Editor),
    ("GET", "/api/pages/{id}/api-config", Role::Viewer),
    ("PUT", "/api/pages/{id}/api-config", Role::Editor),
    ("POST", "/api/pages/{id}/api-config/check", Role::Editor),
    ("GET", "/api/pages/{id}/api-spec", Role::Viewer),
    ("GET", "/api/pages/{id}/backlinks", Role::Viewer),
    ("PUT", "/api/pages/{id}/description", Role::Editor),
    ("GET", "/api/pages/{id}/draft", Role::Viewer),
    ("PUT", "/api/pages/{id}/draft", Role::Editor),
    ("PUT", "/api/pages/{id}/header", Role::Editor),
    ("GET", "/api/pages/{id}/history", Role::Viewer),
    ("GET", "/api/pages/{id}/impact", Role::Viewer),
    ("GET", "/api/pages/{id}/lock", Role::Viewer),
    ("POST", "/api/pages/{id}/lock", Role::Editor),
    ("PUT", "/api/pages/{id}/lock", Role::Editor),
    ("DELETE", "/api/pages/{id}/lock", Role::Editor),
    ("POST", "/api/pages/{id}/lock/take-over", Role::Editor),
    ("POST", "/api/pages/{id}/move", Role::Editor),
    ("GET", "/api/pages/{id}/operations", Role::Viewer),
    ("POST", "/api/pages/{id}/restore", Role::Editor),
    ("GET", "/api/pages/{id}/review", Role::Viewer),
    ("PUT", "/api/pages/{id}/review", Role::Editor),
    ("POST", "/api/pages/{id}/review/done", Role::Editor),
    ("GET", "/api/pages/{id}/url", Role::Viewer),
    ("PUT", "/api/pages/{id}/url", Role::Editor),
    ("POST", "/api/publish", Role::Publisher),
    ("GET", "/api/publish/changes", Role::Viewer),
    ("GET", "/api/publish/checks", Role::Viewer),
    ("POST", "/api/publish/plan", Role::Editor),
    ("POST", "/api/publish/preview", Role::Editor),
    ("GET", "/api/publish/progress", Role::Viewer),
    ("GET", "/api/redirects", Role::Viewer),
    ("POST", "/api/redirects", Role::Editor),
    ("DELETE", "/api/redirects", Role::Editor),
    ("GET", "/api/releases", Role::Viewer),
    ("POST", "/api/releases/{id}/promote", Role::Publisher),
    ("POST", "/api/releases/{id}/rollback", Role::Publisher),
    ("GET", "/api/reports/stale", Role::Viewer),
    ("GET", "/api/revisions/{id}", Role::Viewer),
    ("POST", "/api/jobs/run", Role::Admin),
    ("GET", "/api/scalar.js", Role::Viewer),
    ("GET", "/api/settings", Role::Viewer),
    ("PUT", "/api/settings", Role::Admin),
    ("POST", "/api/settings/check", Role::Admin),
    ("GET", "/api/site", Role::Viewer),
    ("GET", "/api/theme.css", Role::Viewer),
    ("GET", "/api/tokens", Role::Viewer),
    ("POST", "/api/tokens", Role::Viewer),
    ("DELETE", "/api/tokens/{id}", Role::Viewer),
    ("GET", "/api/trash", Role::Viewer),
    ("POST", "/api/trash/{id}/restore", Role::Editor),
    ("POST", "/api/assets", Role::Editor),
    ("GET", "/api/users", Role::Viewer),
    ("PUT", "/api/users/{id}/role", Role::Admin),
  ];

  const ORIGIN: &str = "https://docs.example.com";

  /// The editor's router with a real login, and a session cookie for one user of each role.
  async fn signed_in_app() -> (Router, Vec<(Role, String)>, AppState) {
    use cairn_auth::{AccessRules, Auth, AuthSettings, Oidc, SessionSettings};
    use jiff::SignedDuration;

    let (mut state, _) = test_state_serving(|dir| PublishTarget::SelfHosted(cairn_publish::FolderTarget::new(dir.join("out")))).await;
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
        public_origin: ORIGIN.into(),
        base_path: String::new(),
      },
    );

    let mut cookies = Vec::new();
    // The first user is the admin; the others start as viewers and are given their role.
    for role in [Role::Admin, Role::Publisher, Role::Editor, Role::Viewer] {
      let user = cairn_store::upsert_user(&state.db, "https://idp.example", role.as_str(), &format!("{}@example.com", role.as_str()), role.as_str())
        .await
        .unwrap();
      cairn_store::set_user_role(&state.db, user.id, role).await.unwrap();
      cookies.push((role, state.auth.start_session_for_tests(user.id).await));
    }

    (editor_app(state.auth.clone()).with_state(state.clone()), cookies, state)
  }

  #[test]
  fn every_operation_has_a_minimum_role() {
    let (_, spec) = api();
    let mut missing = Vec::new();

    for (path, item) in &spec.paths.paths {
      for (method, operation) in [("GET", &item.get), ("POST", &item.post), ("PUT", &item.put), ("DELETE", &item.delete), ("PATCH", &item.patch)] {
        if operation.is_some() && !MINIMUM_ROLES.iter().any(|(m, p, _)| *m == method && p == path) {
          missing.push(format!("{method} {path}"));
        }
      }
    }

    assert!(missing.is_empty(), "add these to MINIMUM_ROLES: {missing:?}");
  }

  #[tokio::test]
  async fn each_role_can_call_what_it_may_and_nothing_more() {
    let (app, cookies, _) = signed_in_app().await;
    let (_, spec) = api();

    for (method, path, minimum) in MINIMUM_ROLES {
      // A row for an operation that no longer exists would test nothing.
      assert!(spec.paths.paths.contains_key(*path), "{path} is not an operation any more");
      let uri = path.replace("{id}", &uuid::Uuid::now_v7().to_string()).replace("{key}", "k");
      // A forced take-over needs more than a plain one: this body asks for the plain one.
      let body = if path.ends_with("/take-over") { r#"{"force":false}"# } else { "{}" };

      for (role, cookie) in &cookies {
        let request = Request::builder()
          .method(*method)
          .uri(&uri)
          .header(header::COOKIE, cookie)
          .header(header::ORIGIN, ORIGIN)
          .header(header::CONTENT_TYPE, "application/json")
          .body(Body::from(body))
          .unwrap();
        let status = app.clone().oneshot(request).await.unwrap().status();

        if role < minimum {
          assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path} as {}", role.as_str());
        } else {
          assert_ne!(status, StatusCode::FORBIDDEN, "{method} {path} as {}", role.as_str());
          assert_ne!(status, StatusCode::UNAUTHORIZED, "{method} {path} as {}", role.as_str());
        }
      }
    }
  }

  #[tokio::test]
  async fn a_token_limited_to_viewer_cannot_save_a_draft() {
    let (app, _, state) = signed_in_app().await;
    let admin = cairn_store::upsert_user(&state.db, "https://idp.example", "admin", "admin@example.com", "admin").await.unwrap();
    let (_, secret) = state.auth.issue_token(cairn_auth::CurrentUser(admin.id, admin.role), "CI", Some(Role::Viewer)).await.unwrap();
    let uri = format!("/api/pages/{}/draft", uuid::Uuid::now_v7());

    let request = Request::put(&uri)
      .header(header::AUTHORIZATION, format!("Bearer {secret}"))
      .header(header::CONTENT_TYPE, "application/json")
      .body(Body::from("{}"))
      .unwrap();

    assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::FORBIDDEN);
  }

  #[tokio::test]
  async fn only_an_admin_can_force_a_take_over() {
    let (app, cookies, state) = signed_in_app().await;
    let viewer = cairn_store::upsert_user(&state.db, "https://idp.example", "viewer", "viewer@example.com", "viewer").await.unwrap().id;

    // (role, forcing someone else's lock, expected): forcing one's own lock from another tab is ordinary editing.
    for (role, cookie) in &cookies {
      let holder = match role {
        Role::Viewer => viewer,
        _ => {
          cairn_store::upsert_user(&state.db, "https://idp.example", role.as_str(), &format!("{}@example.com", role.as_str()), role.as_str())
            .await
            .unwrap()
            .id
        }
      };
      for someone_else in [true, false] {
        let title = format!("{}-{someone_else}", role.as_str());
        let page = cairn_store::create_page(&state.db, holder, None, &title, cairn_core::Placement::Last).await.unwrap().id;
        let held_by = if someone_else { viewer } else { holder };
        cairn_store::acquire_lock(&state.db, page, held_by, jiff::Timestamp::now(), None).await.unwrap();

        let request = Request::post(format!("/api/pages/{page}/lock/take-over"))
          .header(header::COOKIE, cookie)
          .header(header::ORIGIN, ORIGIN)
          .header(header::CONTENT_TYPE, "application/json")
          .body(Body::from(r#"{"force":true}"#))
          .unwrap();
        let status = app.clone().oneshot(request).await.unwrap().status();

        let allowed = *role == Role::Admin || (*role != Role::Viewer && !someone_else);
        let expected = if allowed { StatusCode::OK } else { StatusCode::FORBIDDEN };
        assert_eq!(status, expected, "{} someone_else={someone_else}", role.as_str());
      }
    }
  }
}
