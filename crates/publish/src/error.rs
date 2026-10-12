use yourdocs_core::CoreError;
use yourdocs_render::RenderError;
use yourdocs_store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
  /// A page failed to render; nothing was published.
  #[error("page \"{page}\" failed to render: {source}")]
  Render { page: String, source: RenderError },
  #[error(transparent)]
  Store(#[from] StoreError),
  #[error(transparent)]
  Core(#[from] CoreError),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}
