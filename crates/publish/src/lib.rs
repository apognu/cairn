//! Releases, the publish queue and the PublishTarget enum.

#![forbid(unsafe_code)]

mod api;
mod card;
mod changes;
mod cloudflare;
mod content;
mod error;
mod generated;
mod glossary;
mod markdown;
mod pages;
mod publisher;
mod s3;
mod search;
mod site;
mod target;

pub use api::{ApiHosts, Fetched, ResolvedApi, ResolvedSpec, SourceFetcher};
pub use changes::{HotfixPlan, OperationAnchors, PlainLink, check, hotfix_plan, pending};
pub use cloudflare::{CloudflareConfig, CloudflareTarget, Version};
pub use error::{ApiProblem, PublishError};
pub use pages::{PagesConfig, PagesTarget};
pub use publisher::{Bundles, Cleanup, PublishConfig, Publisher, PublisherParts};
pub use s3::{S3Config, S3Target};
pub use target::{Capabilities, Deployed, FolderTarget, OutputFile, Progress, PublishTarget};
