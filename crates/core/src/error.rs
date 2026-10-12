/// Errors shared by every implementation of the traits in this crate.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
  #[error("not found")]
  NotFound,
  /// A save was based on a revision that is no longer the latest.
  #[error("stale save: the page changed since revision {base:?}")]
  Conflict { base: Option<crate::RevisionId> },
  #[error("invalid: {0}")]
  Invalid(String),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}
