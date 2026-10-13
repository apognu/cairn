//! Releases, the publish queue and the PublishTarget enum.

#![forbid(unsafe_code)]

mod api;
mod changes;
mod cloudflare;
mod error;
mod generated;
mod markdown;
mod publisher;
mod search;
mod site;
mod target;

pub use api::{ApiHosts, Fetched, ResolvedApi, ResolvedSpec, SourceFetcher};
pub use changes::{OperationAnchors, check, pending};
pub use cloudflare::{CloudflareConfig, CloudflareTarget, Version};
pub use error::{ApiProblem, PublishError};
pub use publisher::{Bundles, PublishConfig, Publisher, PublisherParts};
pub use target::{Capabilities, Deployed, FolderTarget, OutputFile, PublishTarget};
