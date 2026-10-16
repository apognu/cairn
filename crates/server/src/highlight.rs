use axum::Json;
use cairn_core::CoreError;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

/// Longer code is left plain in the editor: highlighting is a request per pause in typing, and a huge block would hold a blocking thread.
const MAX_CODE: usize = 100_000;

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(highlight))
}

#[derive(Deserialize, ToSchema)]
struct HighlightRequest {
  /// As written in the code block, like `rust` or `tsx`.
  language: String,
  code: String,
}

#[derive(Serialize, ToSchema)]
struct Highlighted {
  /// Runs of styled code, in order; text between them is plain. Empty for a language the highlighter does not know.
  tokens: Vec<CodeToken>,
}

#[derive(Serialize, ToSchema)]
struct CodeToken {
  /// Offsets in UTF-16 code units, as JavaScript counts them.
  from: usize,
  to: usize,
  light: Option<CodeTokenStyle>,
  dark: Option<CodeTokenStyle>,
}

#[derive(Serialize, ToSchema)]
struct CodeTokenStyle {
  color: String,
  italic: bool,
  bold: bool,
}

impl From<cairn_render::TokenStyle> for CodeTokenStyle {
  fn from(style: cairn_render::TokenStyle) -> Self {
    Self {
      color: style.color,
      italic: style.italic,
      bold: style.bold,
    }
  }
}

/// Colors code the way publishing does, for the editor's canvas.
#[utoipa::path(post, path = "/api/highlight", operation_id = "highlightCode", request_body = HighlightRequest, responses(
  (status = 200, body = Highlighted),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all)]
async fn highlight(Json(body): Json<HighlightRequest>) -> Result<Json<Highlighted>, AppError> {
  if body.code.len() > MAX_CODE {
    return Err(AppError::Invalid(vec![format!("code longer than {MAX_CODE} bytes is not highlighted")]));
  }

  let tokens = tokio::task::spawn_blocking(move || cairn_render::highlight_tokens(&body.language, &body.code))
    .await
    .map_err(|e| CoreError::Other(e.into()))?
    .unwrap_or_default();

  Ok(Json(Highlighted {
    tokens: tokens
      .into_iter()
      .map(|t| CodeToken {
        from: t.from,
        to: t.to,
        light: t.light.map(Into::into),
        dark: t.dark.map(Into::into),
      })
      .collect(),
  }))
}
