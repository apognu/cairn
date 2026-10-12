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
    let (status, details) = match &self {
      AppError::NotFound => (StatusCode::NOT_FOUND, vec![]),
      AppError::Invalid(details) => (StatusCode::UNPROCESSABLE_ENTITY, details.clone()),
      AppError::Render(RenderError::Busy) => (StatusCode::SERVICE_UNAVAILABLE, vec![]),
      AppError::Render(RenderError::Timeout) => (StatusCode::GATEWAY_TIMEOUT, vec![]),
      AppError::Store(StoreError::NotFound) => (StatusCode::NOT_FOUND, vec![]),
      AppError::Store(StoreError::Conflict(_)) => (StatusCode::CONFLICT, vec![]),
      AppError::Store(StoreError::Invalid(reason)) => (StatusCode::UNPROCESSABLE_ENTITY, vec![reason.clone()]),
      AppError::Core(CoreError::NotFound) => (StatusCode::NOT_FOUND, vec![]),
      AppError::Core(CoreError::Conflict { .. }) => (StatusCode::CONFLICT, vec![]),
      AppError::Core(CoreError::Invalid(reason)) => (StatusCode::UNPROCESSABLE_ENTITY, vec![reason.clone()]),
      AppError::Publish(PublishError::Blocked(problems)) => (StatusCode::UNPROCESSABLE_ENTITY, problems.iter().map(ToString::to_string).collect()),
      AppError::Publish(PublishError::Render { .. }) => (StatusCode::UNPROCESSABLE_ENTITY, vec![self.to_string()]),
      AppError::Render(_) | AppError::Store(_) | AppError::Core(_) | AppError::Publish(_) => (StatusCode::INTERNAL_SERVER_ERROR, vec![]),
    };

    if status.is_server_error() {
      tracing::error!(error = %self, "request failed");
    }

    // Render failures can carry JS stack traces: log them, never send them to the client.
    let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
      "internal error".to_string()
    } else {
      self.to_string()
    };

    (status, Json(ErrorBody { message, details })).into_response()
  }
}
