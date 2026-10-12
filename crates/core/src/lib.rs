//! Domain types and traits; no I/O.
//!
//! The document model is ProseMirror JSON, and the storage seams (`ContentStore`, `AssetStore`) are traits implemented by other crates.

#![forbid(unsafe_code)]

mod asset;
mod document;
mod error;
mod id;
mod page;
mod storage;

pub use asset::Asset;
pub use document::{Document, Mark, Node};
pub use error::CoreError;
pub use id::{AssetId, PageId, ReleaseId, RevisionId, UserId};
pub use page::{Page, Placement, slugify};
pub use storage::{AssetStore, ContentStore, Draft, Revision};
