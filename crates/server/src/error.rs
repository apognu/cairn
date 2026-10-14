use axum::{
  Json,
  http::StatusCode,
  response::{IntoResponse, Response},
};
use cairn_core::CoreError;
use cairn_publish::PublishError;
use cairn_render::RenderError;
use cairn_store::StoreError;
use serde::Serialize;
use utoipa::ToSchema;

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
  #[error(transparent)]
  Auth(#[from] cairn_auth::AuthError),
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
      AppError::Publish(PublishError::Rejected(problems)) => (StatusCode::UNPROCESSABLE_ENTITY, None, problems.clone()),
      AppError::Publish(PublishError::Stale(reason)) => (StatusCode::CONFLICT, Some(reason.clone()), vec![]),
      AppError::Publish(PublishError::Core(CoreError::Invalid(reason))) => (StatusCode::UNPROCESSABLE_ENTITY, Some(reason.clone()), vec![reason.clone()]),
      AppError::Publish(PublishError::Core(CoreError::NotFound) | PublishError::Store(StoreError::NotFound)) => (StatusCode::NOT_FOUND, None, vec![]),
      AppError::Publish(PublishError::Blocked(problems)) => (StatusCode::UNPROCESSABLE_ENTITY, None, problems.iter().map(ToString::to_string).collect()),
      // Only a page that cannot render is the author's to fix; a full queue or a dead worker is the server's.
      AppError::Publish(PublishError::Render { source, .. }) => match source {
        RenderError::Invalid(_) | RenderError::OutOfMemory => (StatusCode::UNPROCESSABLE_ENTITY, None, vec![self.to_string()]),
        RenderError::Busy => (StatusCode::SERVICE_UNAVAILABLE, None, vec![]),
        RenderError::Timeout => (StatusCode::GATEWAY_TIMEOUT, None, vec![]),
        RenderError::Other(_) => (StatusCode::INTERNAL_SERVER_ERROR, None, vec![]),
      },
      AppError::Auth(cairn_auth::AuthError::Unauthorized) => (StatusCode::UNAUTHORIZED, None, vec![]),
      AppError::Auth(cairn_auth::AuthError::Forbidden(reason)) => (StatusCode::FORBIDDEN, Some(reason.clone()), vec![]),
      AppError::Auth(cairn_auth::AuthError::Store(cairn_store::StoreError::Invalid(reason))) => (StatusCode::UNPROCESSABLE_ENTITY, Some(reason.clone()), vec![reason.clone()]),
      AppError::Render(_) | AppError::Store(_) | AppError::Core(_) | AppError::Publish(_) | AppError::Auth(_) => (StatusCode::INTERNAL_SERVER_ERROR, None, vec![]),
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

/// Axum answers a body, path or query it cannot read with a plain-text sentence of its own. The API promises `ErrorBody` everywhere, so
/// those answers (and only those: ours are already JSON) are wrapped, keeping the original sentence as the detail.
pub(crate) async fn rejections_as_json(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
  let response = next.run(request).await;
  let status = response.status();
  let plain = response
    .headers()
    .get(axum::http::header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .is_some_and(|v| v.starts_with("text/plain"));

  if !(status.is_client_error() && plain) {
    return response;
  }

  let message = match status {
    StatusCode::PAYLOAD_TOO_LARGE => "The request is too large.",
    StatusCode::UNSUPPORTED_MEDIA_TYPE => "The request has an unsupported content type.",
    StatusCode::UNPROCESSABLE_ENTITY => "The request does not have the expected shape.",
    StatusCode::METHOD_NOT_ALLOWED => "This method is not allowed here.",
    _ => "The request is not valid.",
  };
  let Ok(bytes) = axum::body::to_bytes(response.into_body(), 4096).await else {
    return (
      status,
      Json(ErrorBody {
        message: message.into(),
        details: vec![],
      }),
    )
      .into_response();
  };
  let detail = String::from_utf8_lossy(&bytes).trim().to_string();

  (
    status,
    Json(ErrorBody {
      message: message.into(),
      details: (!detail.is_empty()).then_some(detail).into_iter().collect(),
    }),
  )
    .into_response()
}

/// An upload over the limit says what the limit is, which the editor shows as it is.
pub(crate) async fn too_large_upload(limit_mb: usize, request: axum::extract::Request, next: axum::middleware::Next) -> Response {
  let response = next.run(request).await;
  if response.status() != StatusCode::PAYLOAD_TOO_LARGE {
    return response;
  }

  (
    StatusCode::PAYLOAD_TOO_LARGE,
    Json(ErrorBody {
      message: format!("The file is larger than the {limit_mb} MB limit."),
      details: vec![],
    }),
  )
    .into_response()
}

#[cfg(test)]
mod tests {
  use axum::{Router, body::Body, extract::DefaultBodyLimit, http::Request, middleware, routing::post};
  use tower::ServiceExt;

  use super::*;

  async fn body_json(response: Response) -> serde_json::Value {
    serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap()).unwrap()
  }

  #[tokio::test]
  async fn axum_rejections_come_back_as_error_bodies() {
    let app = Router::new()
      .route("/json", post(|Json(_): Json<serde_json::Value>| async { "ok" }))
      .route("/id/{id}", axum::routing::get(|axum::extract::Path(_): axum::extract::Path<uuid::Uuid>| async { "ok" }))
      .route("/big", post(|_: axum::body::Bytes| async { "ok" }).layer(DefaultBodyLimit::max(4)))
      .layer(middleware::from_fn(rejections_as_json));

    // (request, expected status): none of them is JSON the handler can use.
    for (request, status) in [
      (
        Request::post("/json").header("content-type", "application/json").body(Body::from("{nope")).unwrap(),
        StatusCode::BAD_REQUEST,
      ),
      (Request::get("/id/not-a-uuid").body(Body::empty()).unwrap(), StatusCode::BAD_REQUEST),
      (Request::post("/big").body(Body::from("too long")).unwrap(), StatusCode::PAYLOAD_TOO_LARGE),
    ] {
      let response = app.clone().oneshot(request).await.unwrap();
      assert_eq!(response.status(), status);
      assert!(body_json(response).await["message"].is_string());
    }
  }

  #[tokio::test]
  async fn an_oversized_upload_names_the_limit() {
    let app = Router::new()
      .route("/up", post(|_: axum::body::Bytes| async { "ok" }).layer(DefaultBodyLimit::max(4)))
      .layer(middleware::from_fn(|r, n| too_large_upload(25, r, n)))
      .layer(middleware::from_fn(rejections_as_json));

    let response = app.oneshot(Request::post("/up").body(Body::from("too long")).unwrap()).await.unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(body_json(response).await["message"].as_str().unwrap().contains("25 MB"));
  }
}
