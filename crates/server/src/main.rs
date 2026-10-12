//! Axum server: API, editor and (later) the site.

#![forbid(unsafe_code)]

mod app;
mod assets;
mod error;

use std::{
  collections::HashMap,
  env,
  sync::{Arc, RwLock},
  time::Duration,
};

use anyhow::Context;
use yourdocs_render::Pool;

use crate::{
  app::{AppState, router},
  assets::Assets,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
    .init();

  let bind = env::var("BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());

  let script = Assets::get("ssr.js").context("ssr.js is not embedded, run `bun run build` before `cargo build`")?;
  let script = std::str::from_utf8(&script.data).context("ssr.js is not valid UTF-8")?;
  let pool = Pool::new(script, 2, 64, Duration::from_secs(5)).context("failed to start the render pool")?;

  // The spike has no storage yet: start with the fixture so a page exists to look at.
  let mut docs = HashMap::new();
  docs.insert("demo".to_string(), serde_json::from_str(include_str!("../../../fixtures/tabs.json")).context("invalid fixture")?);

  let state = AppState {
    pool: Arc::new(pool),
    docs: Arc::new(RwLock::new(docs)),
  };

  let listener = tokio::net::TcpListener::bind(&bind).await.with_context(|| format!("cannot bind {bind}"))?;
  tracing::info!(%bind, "listening");

  axum::serve(listener, router(state)).await.context("server stopped")
}
