use axum::{Json, extract::State};
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::Release;

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(publish))
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
