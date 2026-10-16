//! Axum server: API, editor and (later) the site.

#![forbid(unsafe_code)]

mod api_pages;
mod app;
mod assets;
mod bundles;
mod config;
mod drafts;
mod editor;
mod error;
mod highlight;
mod history;
mod jobs;
mod library;
mod links;
mod locks;
mod me;
mod pages;
mod proxy;
mod publish;
mod redirects;
mod serving;
mod settings;
mod site;
mod tokens;
mod uploads;
mod urls;

use std::{env, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, bail};
use cairn_render::Pool;

use crate::{
  app::{AppState, dev_router},
  bundles::BundleSet,
};

/// The OpenAPI spec of the HTTP API, as pretty-printed JSON.
pub fn openapi_json() -> anyhow::Result<String> {
  app::api().1.to_pretty_json().context("cannot serialize the OpenAPI spec")
}

/// How requests are authenticated, decided by the command.
enum AuthSetup {
  /// `dev`: everyone is the local user.
  Local,
  /// `serve`: a provider's sign-in.
  Oidc(Box<config::OidcSettings>, Option<config::GithubSettings>, String),
}

/// What both commands run: the stores, the render pool and the publisher, over one data directory.
struct Runtime {
  state: AppState,
  events: cairn_core::LocalBroadcast,
  /// Tells the event streams the server is stopping.
  stop: tokio::sync::watch::Sender<bool>,
}

async fn prepare(
  data_dir: &std::path::Path,
  database_url: Option<&str>,
  assets: &cairn_assets::AssetBackend,
  asset_grace: jiff::SignedDuration,
  site_url: String,
  auth: AuthSetup,
  target: cairn_publish::PublishTarget,
) -> anyhow::Result<Runtime> {
  std::fs::create_dir_all(data_dir).with_context(|| format!("cannot create {}", data_dir.display()))?;
  let db = match database_url {
    Some(url) => cairn_store::connect_url(url).await,
    None => cairn_store::connect(&data_dir.join("cairn.db")).await,
  }
  .context("cannot open the database")?;

  // The runtime image keeps the bundles `build-components` made in /app/bundles; anywhere else the ones in the binary are used.
  let bundles = match env::var_os("BUNDLES_DIR") {
    Some(dir) => BundleSet::load(Some(&PathBuf::from(dir)), true)?,
    None => BundleSet::load(Some(std::path::Path::new("/app/bundles")), false)?,
  };
  // V8 starts with the first preview, check or publish: signing in and editing never wait for it.
  let pool = Pool::lazy(&bundles.ssr_js, 2, 64, Duration::from_secs(5));

  let pool = Arc::new(pool);
  let content = cairn_store::SqlContentStore::new(db.clone());
  let assets = Arc::new(cairn_assets::ObjectAssetStore::open(assets).context("cannot open the asset store")?);

  let events = cairn_core::LocalBroadcast::new();

  let auth = match auth {
    AuthSetup::Local => cairn_auth::Auth::local(),
    AuthSetup::Oidc(settings, github, redirect) => {
      let settings = *settings;
      let auth_settings = cairn_auth::AuthSettings {
        sessions: settings.sessions(redirect.starts_with("https://")),
        access: settings.access.clone(),
        public_origin: config::origin_of(&redirect),
        base_path: EDITOR_PATH.into(),
      };
      let github = github
        .map(|g| {
          cairn_auth::Github::new(
            cairn_auth::GithubConfig {
              client_id: g.client_id,
              client_secret: g.client_secret,
              web_url: g.web_url,
              api_url: g.api_url,
              access: g.access,
            },
            &redirect.replace("/auth/callback", "/auth/github/callback"),
          )
        })
        .transpose()?;
      let oidc = cairn_auth::Oidc::discover(settings.into(), &redirect).await?;
      cairn_auth::Auth::oidc(oidc, github, db.clone(), auth_settings)
    }
  };

  let (stop, stopping) = tokio::sync::watch::channel(false);
  let state = AppState {
    stopping,
    asset_grace,
    editor_dir: bundles.editor_dir.clone().map(Arc::new),
    api_checks: Default::default(),
    scalar_js: Arc::new(bundles.scalar_js.clone()),
    publisher: app::publisher(
      db.clone(),
      content.clone(),
      assets.clone(),
      pool.clone(),
      app::PublishSetup {
        bundles: cairn_publish::Bundles::new(&bundles.ssr_js, bundles.client_js, bundles.site_css, bundles.scalar_js),
        config: cairn_publish::PublishConfig {
          site_url,
          pagefind: env::var("PAGEFIND_BIN").unwrap_or_else(|_| "pagefind".into()),
          api_hosts: api_hosts(),
        },
      },
      events.clone(),
      target,
    )?,
    pool,
    auth,
    content,
    assets,
    db,
  };

  // A site last rendered by other bundles (new components, a new version) is rendered again in the background, with what is live.
  let publisher = state.publisher.clone();
  tokio::spawn(async move {
    match publisher.republish_for_bundles().await {
      Ok(Some(release)) => tracing::info!(release_id = %release.id, "the live site was rendered again for the new bundles"),
      Ok(None) => {}
      Err(error) => tracing::error!(%error, "cannot render the live site again for the new bundles; it is tried again at the next start"),
    }
  });

  tokio::spawn(purge_trash_daily(state.db.clone()));

  Ok(Runtime { state, events, stop })
}

/// How long a deleted page waits in the trash before it is deleted for good.
const TRASH_DAYS: i64 = 90;

/// Empties the trash of pages older than `TRASH_DAYS`, at startup and then once a day.
async fn purge_trash_daily(db: cairn_store::DatabaseConnection) {
  let mut daily = tokio::time::interval(Duration::from_secs(24 * 60 * 60));

  loop {
    daily.tick().await;
    let before = jiff::Timestamp::now() - jiff::SignedDuration::from_hours(24 * TRASH_DAYS);
    match cairn_store::purge_trash(&db, before).await {
      Ok(0) => {}
      Ok(purged) => tracing::info!(purged, "pages deleted for good after {TRASH_DAYS} days in the trash"),
      Err(error) => tracing::error!(%error, "cannot empty the trash; it is tried again tomorrow"),
    }
  }
}

/// Runs the editor for local use: no login, every request is the local user, so it only ever listens on a loopback address.
pub async fn run_dev(bind: SocketAddr, site_bind: SocketAddr, data_dir: PathBuf) -> anyhow::Result<()> {
  for address in [bind, site_bind] {
    if !address.ip().is_loopback() {
      bail!("`dev` has no login and listens on localhost only; {address} is not a loopback address");
    }
  }

  // Without PUBLIC_URL, the address `dev` serves the site on, whichever port it was given.
  let site_url = env::var("PUBLIC_URL").unwrap_or_else(|_| format!("http://{site_bind}")).trim_end_matches('/').to_string();
  let target = cairn_publish::PublishTarget::Folder(cairn_publish::FolderTarget::new(data_dir.clone()));
  let Runtime { state, events, .. } = prepare(
    &data_dir,
    None,
    &cairn_assets::AssetBackend::Local(data_dir.join("assets")),
    jobs::DEFAULT_ASSET_GRACE,
    site_url,
    AuthSetup::Local,
    target,
  )
  .await?;

  let editor = tokio::net::TcpListener::bind(bind).await.with_context(|| format!("cannot bind {bind}"))?;
  let site = tokio::net::TcpListener::bind(site_bind).await.with_context(|| format!("cannot bind {site_bind}"))?;
  tracing::info!(editor = %bind, site = %site_bind, "listening");

  let site_router = site::dev_router(data_dir.join("site"), events);
  tokio::try_join!(axum::serve(editor, dev_router(state)), axum::serve(site, site_router)).context("server stopped")?;

  Ok(())
}

/// Where `serve` mounts the editor, so that every page of the public site keeps its own address.
const EDITOR_PATH: &str = "/_editor";

/// Runs the server for a deployment: sign-in through the OIDC provider, configured from the environment. One listener serves the editor
/// under `/_editor` and, with the self-hosted target, the public site at `/`.
pub async fn run_serve() -> anyhow::Result<()> {
  let config = config::Config::from_env()?;
  let redirect = format!("{}{EDITOR_PATH}/auth/callback", config.public_url);
  let target = match &config.target {
    config::TargetChoice::SelfHosted => cairn_publish::PublishTarget::SelfHosted(cairn_publish::FolderTarget::new(config.data_dir.clone())),
    config::TargetChoice::Folder(dir) => cairn_publish::PublishTarget::Folder(cairn_publish::FolderTarget::new(dir.clone())),
    config::TargetChoice::Pages(pages) => cairn_publish::PublishTarget::Pages(cairn_publish::PagesTarget::new(pages.clone())?),
    config::TargetChoice::S3(s3) => cairn_publish::PublishTarget::S3(cairn_publish::S3Target::new(s3.clone())?),
    config::TargetChoice::Cloudflare(cloudflare) => cairn_publish::PublishTarget::Cloudflare(cairn_publish::CloudflareTarget::new(cloudflare.clone())?),
  };
  let setup = AuthSetup::Oidc(Box::new(config.oidc.clone()), config.github.clone(), redirect);
  let Runtime { state, stop, .. } = prepare(
    &config.data_dir,
    config.database_url.as_deref(),
    &config.assets,
    config.asset_grace,
    config.site_url.clone(),
    setup,
    target,
  )
  .await?;

  let listener = tokio::net::TcpListener::bind(config.bind).await.with_context(|| format!("cannot bind {}", config.bind))?;
  tracing::info!(address = %config.bind, public_url = %config.public_url, "listening");
  // Forwarded headers are believed only from the configured proxies, which needs to know who is connecting.
  let app = serving::serve_router(state).layer(axum::middleware::from_fn_with_state(config.trusted_proxies.clone(), proxy::forwarded));
  axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
    .with_graceful_shutdown(shutdown_signal(stop))
    .await
    .context("server stopped")?;

  Ok(())
}

/// Resolves when the process is asked to stop (SIGTERM from a container runtime or systemd, or Ctrl-C), so requests in flight finish.
/// Without it a container, whose server is process 1, ignores SIGTERM and is killed after a timeout.
async fn shutdown_signal(stop: tokio::sync::watch::Sender<bool>) {
  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      Ok(mut signal) => {
        signal.recv().await;
      }
      Err(_) => std::future::pending().await,
    }
  };

  tokio::select! {
    _ = tokio::signal::ctrl_c() => {}
    _ = terminate => {}
  }
  tracing::info!("stopping");
  // Ends the editors' event streams, which never finish on their own; other requests in flight are waited for.
  stop.send_replace(true);
}

/// GitHub's hosts for API sources. The variables exist for tests and mirrors; a deployment normally sets neither.
fn api_hosts() -> cairn_publish::ApiHosts {
  let defaults = cairn_publish::ApiHosts::default();
  let host = |name: &str, default: String| env::var(name).map(|v| v.trim_end_matches('/').to_string()).unwrap_or(default);

  cairn_publish::ApiHosts {
    github: host("GITHUB_BASE_URL", defaults.github),
    raw: host("RAW_BASE_URL", defaults.raw),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // `dev` has no login, so listening beyond this machine would hand the editor to anyone on the network.
  #[tokio::test]
  async fn dev_refuses_addresses_other_than_loopback() {
    for (editor, site) in [("0.0.0.0:0", "127.0.0.1:0"), ("127.0.0.1:0", "192.168.1.5:0"), ("[::]:0", "127.0.0.1:0")] {
      let result = run_dev(editor.parse().unwrap(), site.parse().unwrap(), std::env::temp_dir().join("cairn-refused")).await;
      let error = result.expect_err("must refuse").to_string();

      assert!(error.contains("loopback"), "{error}");
    }
  }
}
