use futures_util::future::join_all;
use serde_json::Value;
use sha2::{Digest, Sha256};
use yourdocs_apispec::{Fetch, MergeInput, MergeOptions, load, merge};
use yourdocs_core::{ApiConfig, Operation, ResolvedSource};

use super::fetch::SourceFetcher;
use crate::PublishError;

/// An API page's sources fetched, bundled, validated and merged.
pub struct ResolvedSpec {
  pub spec: Value,
  /// The merged document as served at `/_api/<slug>.<hash>.json`.
  pub bytes: Vec<u8>,
  /// Of `bytes`; the file name and the render cache key change with it.
  pub hash: String,
  pub sources: Vec<ResolvedSource>,
  pub operations: Vec<Operation>,
}

/// Other files a spec points to are fetched like the sources themselves.
struct Refs<'a>(&'a SourceFetcher);

impl Fetch for Refs<'_> {
  async fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
    self.0.get(url).await.map(|fetched| fetched.bytes)
  }
}

/// Resolves the sources of one page. The error is a sentence for the author, naming the source that failed.
pub async fn resolve(fetcher: &SourceFetcher, config: &ApiConfig) -> Result<ResolvedSpec, String> {
  if config.sources.is_empty() {
    return Err("it has no sources yet: add the specs it is built from in its settings".into());
  }

  let loaded = join_all(config.sources.iter().map(|source| async move {
    let fetched = fetcher.fetch_source(source).await.map_err(|e| match e {
      PublishError::ApiSource { label, message } => format!("{label}: {message}"),
      other => other.to_string(),
    })?;
    let spec = load(&fetched.bytes, &fetched.url, &Refs(fetcher)).await.map_err(|e| format!("{}: {e}", source.label()))?;

    Ok::<_, String>((fetched, spec))
  }))
  .await;

  // Every failing source is named, not just the first.
  let (loaded, failures): (Vec<_>, Vec<_>) = loaded.into_iter().partition(Result::is_ok);
  if !failures.is_empty() {
    return Err(failures.into_iter().filter_map(Result::err).collect::<Vec<_>>().join("; "));
  }

  let mut inputs = Vec::new();
  let mut sources = Vec::new();
  for (source, (fetched, spec)) in config.sources.iter().zip(loaded.into_iter().filter_map(Result::ok)) {
    let label = source.label();
    inputs.push(MergeInput {
      label: fetched.tag.as_ref().map_or_else(|| label.clone(), |tag| format!("{label} ({tag})")),
      spec,
      prefix: source.prefix().to_string(),
    });
    sources.push(ResolvedSource {
      label,
      url: fetched.url,
      tag: fetched.tag,
      etag: fetched.etag,
    });
  }

  let options = MergeOptions {
    title: config.merge.title.clone(),
    version: config.merge.version.clone(),
    description: config.merge.description.clone(),
    servers: config.display.servers.clone(),
  };
  let merged = merge(inputs, &options).map_err(|e| e.to_string())?;

  let bytes = serde_json::to_vec(&merged.spec).map_err(|e| e.to_string())?;
  let hash = Sha256::digest(&bytes).iter().take(8).map(|b| format!("{b:02x}")).collect();

  Ok(ResolvedSpec {
    spec: merged.spec,
    bytes,
    hash,
    sources,
    operations: merged.operations,
  })
}
