use axum::{Json, extract::State};
use cairn_auth::{CurrentUser, local_user_id};
use cairn_core::User;
use cairn_store::get_user;
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(me))
}

/// The signed-in user.
#[utoipa::path(get, path = "/api/me", operation_id = "getMe", responses(
  (status = 200, body = User),
  (status = 401, body = ErrorBody, description = "not signed in"),
))]
#[instrument(skip_all)]
async fn me(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Result<Json<User>, AppError> {
  // `dev` has no login and no row for its one user.
  if user == local_user_id() {
    return Ok(Json(User {
      id: user,
      email: String::new(),
      name: "Local user".into(),
    }));
  }

  Ok(Json(get_user(&state.db, user).await?))
}
