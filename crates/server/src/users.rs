//! The people who signed in, and their roles.

use axum::{
  Json,
  extract::{Path, State},
};
use cairn_auth::{Admin, CurrentUser, local_user_id};
use cairn_core::{Role, User, UserId};
use cairn_store::{list_users, set_user_role};
use serde::Deserialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(users)).routes(routes!(set_role))
}

/// Everyone who signed in, by name, with their role: the owners a page can have, and the people an admin gives roles to.
#[utoipa::path(get, path = "/api/users", operation_id = "listUsers", responses(
  (status = 200, body = Vec<User>),
))]
#[instrument(skip_all)]
async fn users(State(state): State<AppState>, CurrentUser(user, _): CurrentUser) -> Result<Json<Vec<User>>, AppError> {
  let mut users = list_users(&state.db).await?;
  // `dev` has no login and no row for its one user.
  if user == local_user_id() {
    users.insert(0, crate::me::local_user());
  }

  Ok(Json(users))
}

#[derive(Deserialize, ToSchema)]
struct SetRole {
  role: Role,
}

/// Gives someone a role. It applies to their next request, and to their API tokens. The site always keeps at least one admin.
#[utoipa::path(put, path = "/api/users/{id}/role", operation_id = "setUserRole", params(("id" = UserId, Path)), request_body = SetRole, responses(
  (status = 200, body = User),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "it would leave the site without an admin"),
))]
#[instrument(skip_all, fields(user_id = %id))]
async fn set_role(State(state): State<AppState>, Admin(by): Admin, Path(id): Path<UserId>, Json(body): Json<SetRole>) -> Result<Json<User>, AppError> {
  let user = set_user_role(&state.db, id, body.role).await?;
  tracing::info!(user_id = %id, by = %by, role = body.role.as_str(), "role changed");

  Ok(Json(user))
}
