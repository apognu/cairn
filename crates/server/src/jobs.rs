//! Housekeeping that an external scheduler asks for, so an idle server never wakes itself up.

use axum::{Json, extract::State};
use cairn_auth::Admin;
use cairn_core::StalePage;
use jiff::Timestamp;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

/// How long an upload nothing uses is kept, and how long a saved version holds the images it shows, unless `ASSET_GRACE` says otherwise.
pub(crate) const DEFAULT_ASSET_GRACE: &str = "720h";

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(run))
}

/// What one run did.
#[derive(Serialize, ToSchema)]
struct JobsReport {
  /// Uploads that no draft, setting, published release or recent version used any more.
  assets_deleted: usize,
  /// Stored files deleted: those of the uploads above, and files no upload owns.
  files_deleted: usize,
  /// The pages whose review date has come, the most overdue first.
  stale_pages: Vec<StalePage>,
}

/// Runs the periodic jobs: deletes the uploads nothing uses any more, and reports the pages due for a review. Meant for a scheduler (cron, Cloud Scheduler) with an API token.
#[utoipa::path(post, path = "/api/jobs/run", operation_id = "runJobs", responses(
  (status = 200, body = JobsReport),
  (status = 401, body = ErrorBody, description = "not signed in and no API token"),
))]
#[instrument(skip_all)]
async fn run(State(state): State<AppState>, Admin(_): Admin) -> Result<Json<JobsReport>, AppError> {
  let cleanup = state.publisher.clean_assets(Timestamp::now() - state.asset_grace).await?;

  Ok(Json(JobsReport {
    assets_deleted: cleanup.assets,
    files_deleted: cleanup.files,
    stale_pages: crate::reviews::due(&state).await?,
  }))
}
