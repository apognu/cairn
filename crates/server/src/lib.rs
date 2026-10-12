//! Axum server: API, editor and (later) the site.

#![forbid(unsafe_code)]

mod app;
mod assets;
mod drafts;
mod editor;
mod error;
mod pages;
mod publish;
mod settings;
mod site;
mod uploads;
mod urls;

use std::{
  collections::HashMap,
  env,
  net::SocketAddr,
  path::PathBuf,
  sync::{Arc, RwLock},
  time::Duration,
};

use anyhow::{Context, bail};
use yourdocs_render::Pool;

use crate::{
  app::{AppState, router},
  assets::Assets,
};

/// The OpenAPI spec of the HTTP API, as pretty-printed JSON.
pub fn openapi_json() -> anyhow::Result<String> {
  app::api().1.to_pretty_json().context("cannot serialize the OpenAPI spec")
}

/// Runs the editor for local use: no login, every request is the local user, so it only ever listens on a loopback address.
pub async fn run_dev(bind: SocketAddr, site_bind: SocketAddr, data_dir: PathBuf) -> anyhow::Result<()> {
  for address in [bind, site_bind] {
    if !address.ip().is_loopback() {
      bail!("`dev` has no login and listens on localhost only; {address} is not a loopback address");
    }
  }

  std::fs::create_dir_all(&data_dir).with_context(|| format!("cannot create {}", data_dir.display()))?;
  let db = yourdocs_store::connect(&data_dir.join("yourdocs.db")).await.context("cannot open the database")?;

  let script = Assets::get("ssr.js").context("ssr.js is not embedded, run `bun run build` before `cargo build`")?;
  let script = std::str::from_utf8(&script.data).context("ssr.js is not valid UTF-8")?;
  let pool = Pool::new(script, 2, 64, Duration::from_secs(5)).context("failed to start the render pool")?;

  // The spike has no storage yet: start with the fixture so a page exists to look at.
  let mut docs = HashMap::new();
  for (name, source) in [
    ("demo", include_str!("../../../fixtures/tabs.json")),
    ("article", include_str!("../../../fixtures/article.json")),
    ("features", include_str!("../../../fixtures/features.json")),
  ] {
    docs.insert(name.to_string(), serde_json::from_str(source).with_context(|| format!("invalid fixture {name}"))?);
  }

  let pool = Arc::new(pool);
  let content = yourdocs_store::SqlContentStore::new(db.clone());
  let assets = Arc::new(yourdocs_assets::LocalAssetStore::new(&data_dir.join("assets")).context("cannot open the asset store")?);

  let events = yourdocs_core::LocalBroadcast::new();

  let state = AppState {
    publisher: app::publisher(
      db.clone(),
      content.clone(),
      assets.clone(),
      pool.clone(),
      &data_dir,
      yourdocs_publish::PublishConfig {
        site_url: env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:3001".into()).trim_end_matches('/').to_string(),
        pagefind: env::var("PAGEFIND_BIN").unwrap_or_else(|_| "pagefind".into()),
      },
      events.clone(),
    )?,
    pool,
    docs: Arc::new(RwLock::new(docs)),
    content,
    assets,
    db,
  };

  let editor = tokio::net::TcpListener::bind(bind).await.with_context(|| format!("cannot bind {bind}"))?;
  let site = tokio::net::TcpListener::bind(site_bind).await.with_context(|| format!("cannot bind {site_bind}"))?;
  tracing::info!(editor = %bind, site = %site_bind, "listening");

  let site_router = site::router(data_dir.join("site"), events);
  tokio::try_join!(axum::serve(editor, router(state)), axum::serve(site, site_router)).context("server stopped")?;

  Ok(())
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
