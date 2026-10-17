use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
};
use cairn_auth::{Auth, CurrentUser, InteractiveUser};
use cairn_core::{ApiToken, Role};
use cairn_store::{delete_api_token, list_api_tokens};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(list, create)).routes(routes!(revoke))
}

#[derive(Deserialize, ToSchema)]
struct NewToken {
  name: String,
  /// The most the token may do, up to your own role; none gives it your role.
  #[serde(default)]
  role: Option<Role>,
}

/// A new token with its secret, which is shown this once.
#[derive(Serialize, ToSchema)]
struct CreatedToken {
  token: ApiToken,
  /// Sent as `Authorization: Bearer <secret>`.
  secret: String,
}

/// The signed-in user's API tokens. Tokens can only be managed from a browser session, never with a token.
#[utoipa::path(get, path = "/api/tokens", operation_id = "listTokens", responses(
  (status = 200, body = Vec<ApiToken>),
  (status = 403, body = ErrorBody, description = "called with an API token"),
))]
#[instrument(skip_all)]
async fn list(State(state): State<AppState>, InteractiveUser(CurrentUser(user, _)): InteractiveUser) -> Result<Json<Vec<ApiToken>>, AppError> {
  Ok(Json(list_api_tokens(&state.db, user).await?))
}

/// Makes a token for scripts and CI. It acts with its own role if it has one, and never above its owner's current role.
#[utoipa::path(post, path = "/api/tokens", operation_id = "createToken", request_body = NewToken, responses(
  (status = 201, body = CreatedToken),
  (status = 403, body = ErrorBody, description = "called with an API token, in `dev`, or with a role above yours"),
  (status = 422, body = ErrorBody, description = "no name"),
))]
#[instrument(skip_all)]
async fn create(State(auth): State<Auth>, InteractiveUser(user): InteractiveUser, Json(new): Json<NewToken>) -> Result<(StatusCode, Json<CreatedToken>), AppError> {
  let (token, secret) = auth.issue_token(user, &new.name, new.role).await?;

  Ok((StatusCode::CREATED, Json(CreatedToken { token, secret })))
}

/// Deletes a token; scripts using it stop working at once.
#[utoipa::path(delete, path = "/api/tokens/{id}", operation_id = "revokeToken", params(("id" = String, Path)), responses(
  (status = 204, description = "deleted"),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn revoke(State(state): State<AppState>, InteractiveUser(CurrentUser(user, _)): InteractiveUser, Path(id): Path<uuid::Uuid>) -> Result<StatusCode, AppError> {
  delete_api_token(&state.db, user, id).await?;

  Ok(StatusCode::NO_CONTENT)
}
