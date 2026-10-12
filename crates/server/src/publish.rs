use axum::{Json, extract::State};
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{Pending, Release};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(publish)).routes(routes!(changes))
}

/// Publishes the latest saved draft of every page as a new release.
#[utoipa::path(post, path = "/api/publish", operation_id = "publish", responses(
  (status = 200, body = Release),
  (status = 422, body = ErrorBody, description = "a page could not be rendered; nothing was published"),
))]
#[instrument(skip_all)]
async fn publish(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Result<Json<Release>, AppError> {
  Ok(Json(state.publisher.publish(user).await?))
}

/// What the next publish would change compared with the live site, and the live URLs it would break.
#[utoipa::path(get, path = "/api/publish/changes", operation_id = "pendingChanges", responses((status = 200, body = Pending)))]
#[instrument(skip_all)]
async fn changes(State(state): State<AppState>) -> Result<Json<Pending>, AppError> {
  Ok(Json(yourdocs_publish::pending(&state.db).await?))
}
