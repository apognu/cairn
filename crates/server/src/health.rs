//! Probes for container platforms and load balancers, open to everyone at `/-/`: whether the process answers, and whether it can serve.

use axum::{
  Router,
  extract::State,
  http::{StatusCode, header},
  response::IntoResponse,
  routing::get,
};

use crate::app::AppState;

pub(crate) fn routes() -> Router<AppState> {
  Router::new().route("/-/healthz", get(healthz)).route("/-/readyz", get(readyz))
}

/// Alive as long as it answers: nothing else is checked, so a slow database never gets the process restarted.
async fn healthz() -> impl IntoResponse {
  answer(StatusCode::OK, "ok")
}

/// Ready while the database answers and the server is not stopping, so a load balancer drains it during a graceful shutdown. The asset
/// store and V8 are left out: one is a network call on every probe, the other starts on first use.
async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
  if *state.stopping.borrow() {
    return answer(StatusCode::SERVICE_UNAVAILABLE, "stopping");
  }
  if let Err(error) = cairn_store::ping(&state.db).await {
    tracing::warn!(error = %error, "readiness check failed");
    return answer(StatusCode::SERVICE_UNAVAILABLE, "database unreachable");
  }

  answer(StatusCode::OK, "ok")
}

fn answer(status: StatusCode, text: &'static str) -> impl IntoResponse {
  (status, [(header::CACHE_CONTROL, "no-store")], text)
}
