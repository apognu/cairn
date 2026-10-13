use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::{Auth, InteractiveUser};
use yourdocs_core::ApiToken;
use yourdocs_store::{delete_api_token, list_api_tokens};

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
async fn list(State(state): State<AppState>, InteractiveUser(user): InteractiveUser) -> Result<Json<Vec<ApiToken>>, AppError> {
  Ok(Json(list_api_tokens(&state.db, user).await?))
}

/// Makes a token for scripts and CI. It has the same access as its owner.
#[utoipa::path(post, path = "/api/tokens", operation_id = "createToken", request_body = NewToken, responses(
  (status = 201, body = CreatedToken),
  (status = 403, body = ErrorBody, description = "called with an API token, or in `dev`"),
  (status = 422, body = ErrorBody, description = "no name"),
))]
#[instrument(skip_all)]
async fn create(State(auth): State<Auth>, InteractiveUser(user): InteractiveUser, Json(new): Json<NewToken>) -> Result<(StatusCode, Json<CreatedToken>), AppError> {
  let (token, secret) = auth.issue_token(user, &new.name).await?;

  Ok((StatusCode::CREATED, Json(CreatedToken { token, secret })))
}

/// Deletes a token; scripts using it stop working at once.
#[utoipa::path(delete, path = "/api/tokens/{id}", operation_id = "revokeToken", params(("id" = String, Path)), responses(
  (status = 204, description = "deleted"),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn revoke(State(state): State<AppState>, InteractiveUser(user): InteractiveUser, Path(id): Path<uuid::Uuid>) -> Result<StatusCode, AppError> {
  delete_api_token(&state.db, user, id).await?;

  Ok(StatusCode::NO_CONTENT)
}
