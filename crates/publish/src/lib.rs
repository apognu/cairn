//! Releases, the publish queue and the PublishTarget enum.

#![forbid(unsafe_code)]

mod error;
mod publisher;
mod site;
mod target;

pub use error::PublishError;
pub use publisher::{Bundles, Publisher};
pub use target::{Capabilities, FolderTarget, OutputFile, PublishTarget};
