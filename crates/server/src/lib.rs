//! Axum server: API, editor and (later) the site.

#![forbid(unsafe_code)]

mod app;
mod assets;
mod editor;
mod error;
mod pages;

use std::{
  collections::HashMap,
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
pub async fn run_dev(bind: SocketAddr, data_dir: PathBuf) -> anyhow::Result<()> {
  if !bind.ip().is_loopback() {
    bail!("`dev` has no login and listens on localhost only; {bind} is not a loopback address");
  }

  std::fs::create_dir_all(&data_dir).with_context(|| format!("cannot create {}", data_dir.display()))?;
  let db = yourdocs_store::connect(&data_dir.join("yourdocs.db")).await.context("cannot open the database")?;

  let script = Assets::get("ssr.js").context("ssr.js is not embedded, run `bun run build` before `cargo build`")?;
  let script = std::str::from_utf8(&script.data).context("ssr.js is not valid UTF-8")?;
  let pool = Pool::new(script, 2, 64, Duration::from_secs(5)).context("failed to start the render pool")?;

  // The spike has no storage yet: start with the fixture so a page exists to look at.
  let mut docs = HashMap::new();
  for (name, source) in [("demo", include_str!("../../../fixtures/tabs.json")), ("article", include_str!("../../../fixtures/article.json"))] {
    docs.insert(name.to_string(), serde_json::from_str(source).with_context(|| format!("invalid fixture {name}"))?);
  }

  let state = AppState {
    pool: Arc::new(pool),
    docs: Arc::new(RwLock::new(docs)),
    db,
  };

  let listener = tokio::net::TcpListener::bind(bind).await.with_context(|| format!("cannot bind {bind}"))?;
  tracing::info!(%bind, "listening");

  axum::serve(listener, router(state)).await.context("server stopped")
}
