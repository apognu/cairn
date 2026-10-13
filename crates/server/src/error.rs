use axum::{
  Json,
  http::StatusCode,
  response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;
use yourdocs_core::CoreError;
use yourdocs_publish::PublishError;
use yourdocs_render::RenderError;
use yourdocs_store::StoreError;

/// The body of every error response.
#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
  pub message: String,
  pub details: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
  #[error("not found")]
  NotFound,
  #[error("invalid document")]
  Invalid(Vec<String>),
  #[error(transparent)]
  Render(#[from] RenderError),
  #[error(transparent)]
  Store(#[from] StoreError),
  #[error(transparent)]
  Core(#[from] CoreError),
  #[error(transparent)]
  Publish(#[from] PublishError),
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    // Errors the user can act on carry a sentence of their own, sent as the message.
    let (status, reason, details) = match &self {
      AppError::NotFound => (StatusCode::NOT_FOUND, None, vec![]),
      AppError::Invalid(details) => (StatusCode::UNPROCESSABLE_ENTITY, None, details.clone()),
      AppError::Render(RenderError::Busy) => (StatusCode::SERVICE_UNAVAILABLE, None, vec![]),
      AppError::Render(RenderError::Timeout) => (StatusCode::GATEWAY_TIMEOUT, None, vec![]),
      AppError::Store(StoreError::NotFound) => (StatusCode::NOT_FOUND, None, vec![]),
      AppError::Store(StoreError::Conflict(reason)) => (StatusCode::CONFLICT, Some(reason.clone()), vec![]),
      AppError::Store(StoreError::Invalid(reason)) => (StatusCode::UNPROCESSABLE_ENTITY, Some(reason.clone()), vec![reason.clone()]),
      AppError::Core(CoreError::NotFound) => (StatusCode::NOT_FOUND, None, vec![]),
      AppError::Core(CoreError::Conflict { .. }) => (StatusCode::CONFLICT, None, vec![]),
      AppError::Core(CoreError::Invalid(reason)) => (StatusCode::UNPROCESSABLE_ENTITY, Some(reason.clone()), vec![reason.clone()]),
      AppError::Publish(PublishError::Api(problems)) => (StatusCode::UNPROCESSABLE_ENTITY, None, problems.iter().map(ToString::to_string).collect()),
      AppError::Publish(PublishError::ApiSource { .. }) => (StatusCode::UNPROCESSABLE_ENTITY, None, vec![self.to_string()]),
      AppError::Publish(PublishError::Blocked(problems)) => (StatusCode::UNPROCESSABLE_ENTITY, None, problems.iter().map(ToString::to_string).collect()),
      // Only a page that cannot render is the author's to fix; a full queue or a dead worker is the server's.
      AppError::Publish(PublishError::Render { source, .. }) => match source {
        RenderError::Invalid(_) => (StatusCode::UNPROCESSABLE_ENTITY, None, vec![self.to_string()]),
        RenderError::Busy => (StatusCode::SERVICE_UNAVAILABLE, None, vec![]),
        RenderError::Timeout => (StatusCode::GATEWAY_TIMEOUT, None, vec![]),
        RenderError::Other(_) => (StatusCode::INTERNAL_SERVER_ERROR, None, vec![]),
      },
      AppError::Render(_) | AppError::Store(_) | AppError::Core(_) | AppError::Publish(_) => (StatusCode::INTERNAL_SERVER_ERROR, None, vec![]),
    };

    if status.is_server_error() {
      tracing::error!(error = %self, "request failed");
    }

    // Render failures can carry JS stack traces: log them, never send them to the client.
    let message = match (status, reason) {
      (StatusCode::INTERNAL_SERVER_ERROR, _) => "internal error".to_string(),
      (_, Some(reason)) => reason,
      (_, None) => self.to_string(),
    };

    (status, Json(ErrorBody { message, details })).into_response()
  }
}
