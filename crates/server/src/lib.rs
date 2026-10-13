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
mod history;
mod library;
mod links;
mod pages;
mod publish;
mod redirects;
mod settings;
mod site;
mod uploads;
mod urls;

use std::{env, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, bail};
use yourdocs_render::Pool;

use crate::{
  app::{AppState, router},
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
  Oidc(config::OidcSettings, String),
}

/// What both commands run: the stores, the render pool and the publisher, over one data directory.
struct Runtime {
  state: AppState,
  events: yourdocs_core::LocalBroadcast,
}

async fn prepare(data_dir: &std::path::Path, site_url: String, auth: AuthSetup) -> anyhow::Result<Runtime> {
  std::fs::create_dir_all(data_dir).with_context(|| format!("cannot create {}", data_dir.display()))?;
  let db = yourdocs_store::connect(&data_dir.join("yourdocs.db")).await.context("cannot open the database")?;

  // The runtime image keeps the bundles `build-components` made in /app/bundles; anywhere else the ones in the binary are used.
  let bundles = match env::var_os("BUNDLES_DIR") {
    Some(dir) => BundleSet::load(Some(&PathBuf::from(dir)), true)?,
    None => BundleSet::load(Some(std::path::Path::new("/app/bundles")), false)?,
  };
  let pool = Pool::new(&bundles.ssr_js, 2, 64, Duration::from_secs(5)).context("failed to start the render pool")?;

  let pool = Arc::new(pool);
  let content = yourdocs_store::SqlContentStore::new(db.clone());
  let assets = Arc::new(yourdocs_assets::LocalAssetStore::new(&data_dir.join("assets")).context("cannot open the asset store")?);

  let events = yourdocs_core::LocalBroadcast::new();

  let auth = match auth {
    AuthSetup::Local => yourdocs_auth::Auth::local(),
    AuthSetup::Oidc(settings, redirect) => {
      let oidc = yourdocs_auth::Oidc::discover(settings.into(), &redirect).await?;
      yourdocs_auth::Auth::oidc(oidc, db.clone())
    }
  };

  let state = AppState {
    editor_dir: bundles.editor_dir.clone().map(Arc::new),
    api_checks: Default::default(),
    scalar_js: Arc::new(bundles.scalar_js.clone()),
    publisher: app::publisher(
      db.clone(),
      content.clone(),
      assets.clone(),
      pool.clone(),
      data_dir,
      app::PublishSetup {
        bundles: yourdocs_publish::Bundles::new(&bundles.ssr_js, bundles.client_js, bundles.site_css, bundles.scalar_js),
        config: yourdocs_publish::PublishConfig {
          site_url,
          pagefind: env::var("PAGEFIND_BIN").unwrap_or_else(|_| "pagefind".into()),
          api_hosts: api_hosts(),
        },
      },
      events.clone(),
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

  Ok(Runtime { state, events })
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
  let Runtime { state, events } = prepare(&data_dir, site_url, AuthSetup::Local).await?;

  let editor = tokio::net::TcpListener::bind(bind).await.with_context(|| format!("cannot bind {bind}"))?;
  let site = tokio::net::TcpListener::bind(site_bind).await.with_context(|| format!("cannot bind {site_bind}"))?;
  tracing::info!(editor = %bind, site = %site_bind, "listening");

  let site_router = site::router(data_dir.join("site"), events);
  tokio::try_join!(axum::serve(editor, router(state)), axum::serve(site, site_router)).context("server stopped")?;

  Ok(())
}

/// Runs the server for a deployment: sign-in through the OIDC provider, configured from the environment.
pub async fn run_serve() -> anyhow::Result<()> {
  let config = config::Config::from_env()?;
  let redirect = format!("{}/auth/callback", config.public_url);
  let Runtime { state, .. } = prepare(&config.data_dir, config.public_url.clone(), AuthSetup::Oidc(config.oidc.clone(), redirect)).await?;

  let listener = tokio::net::TcpListener::bind(config.bind).await.with_context(|| format!("cannot bind {}", config.bind))?;
  tracing::info!(address = %config.bind, public_url = %config.public_url, "listening");
  axum::serve(listener, router(state)).await.context("server stopped")?;

  Ok(())
}

/// GitHub's hosts for API sources. The variables exist for tests and mirrors; a deployment normally sets neither.
fn api_hosts() -> yourdocs_publish::ApiHosts {
  let defaults = yourdocs_publish::ApiHosts::default();
  let host = |name: &str, default: String| env::var(name).map(|v| v.trim_end_matches('/').to_string()).unwrap_or(default);

  yourdocs_publish::ApiHosts {
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
      let result = run_dev(editor.parse().unwrap(), site.parse().unwrap(), std::env::temp_dir().join("yourdocs-refused")).await;
      let error = result.expect_err("must refuse").to_string();

      assert!(error.contains("loopback"), "{error}");
    }
  }
}
