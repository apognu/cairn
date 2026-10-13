//! Fetching and resolving the public OpenAPI specs of API reference pages.

mod fetch;

pub use fetch::{ApiHosts, Fetched, SourceFetcher};
