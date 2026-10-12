use axum::{
  Json,
  http::StatusCode,
  response::{IntoResponse, Response},
};
use serde_json::json;
use yourdocs_render::RenderError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
  #[error("document not found")]
  NotFound,
  #[error("invalid document")]
  Invalid(Vec<String>),
  #[error(transparent)]
  Render(#[from] RenderError),
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let (status, details) = match &self {
      AppError::NotFound => (StatusCode::NOT_FOUND, vec![]),
      AppError::Invalid(details) => (StatusCode::UNPROCESSABLE_ENTITY, details.clone()),
      AppError::Render(RenderError::Busy) => (StatusCode::SERVICE_UNAVAILABLE, vec![]),
      AppError::Render(RenderError::Timeout) => (StatusCode::GATEWAY_TIMEOUT, vec![]),
      AppError::Render(_) => (StatusCode::INTERNAL_SERVER_ERROR, vec![]),
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

    (status, Json(json!({ "message": message, "details": details }))).into_response()
  }
}
