use yourdocs_core::{CoreError, Problem};
use yourdocs_render::RenderError;
use yourdocs_store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
  /// A page failed to render; nothing was published.
  #[error("page \"{page}\" failed to render: {source}")]
  Render { page: String, source: RenderError },
  /// The pre-publish checks found problems; nothing was published.
  #[error("{}", summary(.0))]
  Blocked(Vec<Problem>),
  #[error(transparent)]
  Store(#[from] StoreError),
  #[error(transparent)]
  Core(#[from] CoreError),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}

fn summary(problems: &[Problem]) -> String {
  let list: Vec<String> = problems.iter().map(ToString::to_string).collect();

  format!(
    "{} stop{} the publish: {}",
    if problems.len() == 1 { "1 problem".to_string() } else { format!("{} problems", problems.len()) },
    if problems.len() == 1 { "s" } else { "" },
    list.join("; ")
  )
}
