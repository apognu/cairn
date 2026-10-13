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
  /// A spec of an API reference could not be fetched; nothing was published.
  #[error("API source {label}: {message}")]
  ApiSource { label: String, message: String },
  /// API reference pages whose specs could not be resolved; nothing was published.
  #[error("{}", api_summary(.0))]
  Api(Vec<ApiProblem>),
  #[error(transparent)]
  Store(#[from] StoreError),
  #[error(transparent)]
  Core(#[from] CoreError),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}

/// One API page that could not be built, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiProblem {
  pub page_title: String,
  pub message: String,
}

impl std::fmt::Display for ApiProblem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "API reference “{}”: {}", self.page_title, self.message)
  }
}

fn api_summary(problems: &[ApiProblem]) -> String {
  let list: Vec<String> = problems.iter().map(ToString::to_string).collect();

  format!("the API specs could not be resolved, nothing was published. {}", list.join("; "))
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
