//! Releases, the publish queue and the PublishTarget enum.

#![forbid(unsafe_code)]

mod api;
mod changes;
mod error;
mod generated;
mod markdown;
mod publisher;
mod search;
mod site;
mod target;

pub use api::{ApiHosts, Fetched, SourceFetcher};
pub use changes::{check, pending};
pub use error::PublishError;
pub use publisher::{Bundles, PublishConfig, Publisher, PublisherParts};
pub use target::{Capabilities, FolderTarget, OutputFile, PublishTarget};
