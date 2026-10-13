#[derive(Debug, thiserror::Error)]
pub enum ApiSpecError {
  /// The bytes are neither JSON nor YAML.
  #[error("not valid JSON or YAML: {0}")]
  Parse(String),
  /// A file a spec points to could not be fetched.
  #[error("{url}: {message}")]
  Fetch { url: String, message: String },
  /// The document is not an OpenAPI 3.0 or 3.1 document, or a reference does not lead anywhere.
  #[error("{0}")]
  Invalid(String),
  /// A spec that points to too many files, or too deep.
  #[error("{0}")]
  Limit(String),
}
