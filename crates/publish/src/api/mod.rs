//! Fetching and resolving the public OpenAPI specs of API reference pages.

mod fetch;
mod resolve;
#[cfg(test)]
pub(crate) mod testing;

pub use fetch::{ApiHosts, Fetched, SourceFetcher};
pub use resolve::{ResolvedSpec, resolve};

use yourdocs_core::{ApiConfig, Operation, PageId, ResolvedSource};

/// An API page ready to be rendered: its merged spec and what the release records about it.
#[derive(Debug, Clone)]
pub struct ResolvedApi {
  pub page_id: PageId,
  pub bytes: Vec<u8>,
  pub hash: String,
  pub sources: Vec<ResolvedSource>,
  pub operations: Vec<Operation>,
  /// The page's settings the spec was resolved with; the display settings come from here too.
  pub config: ApiConfig,
}

impl ResolvedApi {
  /// Where the merged spec is served: `/guide/orders/` has `_api/guide-orders.<hash>.json`.
  pub fn spec_path(&self, page_url: &str) -> String {
    let slug = page_url.trim_matches('/').replace('/', "-");

    format!("_api/{}.{}.json", if slug.is_empty() { "index" } else { &slug }, self.hash)
  }

  /// Where the asset store keeps the spec, so a release can be built again without the network.
  pub fn store_key(hash: &str) -> String {
    format!("api-specs/{hash}.json")
  }
}
