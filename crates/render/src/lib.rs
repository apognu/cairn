//! Renders documents to HTML in embedded V8.

#![forbid(unsafe_code)]

mod engine;
mod highlight;
mod platform;
mod pool;
mod shell;

pub use highlight::highlight_css;
pub use pool::Pool;
pub use shell::{Shell, document};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
  /// The document or its input cannot be rendered: the author can fix it. Infrastructure failures are `Other`.
  #[error("render failed: {0}")]
  Invalid(String),
  #[error("render queue is full")]
  Busy,
  /// The render ran past its time and was stopped; the worker is fine and takes the next job.
  #[error("render timed out")]
  Timeout,
  /// The render used more memory than an isolate may have and was stopped; the worker takes the next job with a fresh isolate.
  #[error("render ran out of memory")]
  OutOfMemory,
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}
