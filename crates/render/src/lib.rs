//! Renders documents to HTML in embedded V8.

#![forbid(unsafe_code)]

mod platform;
mod pool;

use ssr_rs::Ssr;

pub use pool::Pool;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
  #[error("render failed: {0}")]
  Invalid(String),
  #[error("render queue is full")]
  Busy,
  #[error("render timed out")]
  Timeout,
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}

/// Calls `SSR.render(doc)` (ssr_rs takes the exported object as entry point) from a bundled `ssr.js`; one instance per thread because V8 isolates are `!Send`.
pub(crate) struct Renderer {
  ssr: Ssr<'static, 'static>,
}

impl Renderer {
  pub(crate) fn new(script: &str) -> Result<Self, RenderError> {
    let ssr = Ssr::from(script.to_string(), "SSR").map_err(|e| RenderError::Invalid(e.to_string()))?;

    Ok(Self { ssr })
  }

  pub(crate) fn render(&mut self, doc: &serde_json::Value) -> Result<String, RenderError> {
    let params = serde_json::to_string(doc).map_err(|e| RenderError::Other(e.into()))?;

    self.ssr.render_to_string(Some(&params)).map_err(|e| RenderError::Invalid(e.to_string()))
  }
}
