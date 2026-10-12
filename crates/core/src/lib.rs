//! Domain types and traits; no I/O.
//!
//! The document model is ProseMirror JSON, and the storage seams (`ContentStore`, `AssetStore`) are traits implemented by other crates.

#![forbid(unsafe_code)]

mod asset;
mod broadcast;
mod document;
mod error;
mod id;
mod links;
mod nav;
mod page;
mod redirect;
mod release;
mod settings;
mod storage;

pub use asset::Asset;
pub use broadcast::{Broadcast, Event, LocalBroadcast};
pub use document::{Document, Mark, Node};
pub use error::CoreError;
pub use id::{AssetId, PageId, ReleaseId, RevisionId, UserId};
pub use links::{LinkKind, PageLink, extract_links};
pub use nav::{header_pages, header_section, in_sidebar, sidebar_root};
pub use page::{Page, PageKind, Placement, page_urls, slugify};
pub use redirect::{BrokenLink, Change, ChangeKind, Inactive, Live, LostUrl, OldUrls, Pending, Redirect, RedirectOrigin, effective_redirects, pending_changes, resolve_redirect};
pub use release::{Release, ReleaseStatus};
pub use settings::Settings;
pub use storage::{AssetStore, ContentStore, Draft, Revision, original_key};
