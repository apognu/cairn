//! Domain types and traits; no I/O.
//!
//! The document model is ProseMirror JSON, and the storage seams (`ContentStore`, `AssetStore`) are traits implemented by other crates.

#![forbid(unsafe_code)]

mod api;
mod asset;
mod broadcast;
mod checks;
mod document;
mod error;
mod find;
mod hotfix;
mod id;
mod library;
mod links;
mod lock;
mod nav;
mod page;
mod redirect;
mod release;
mod review;
mod settings;
mod storage;
mod user;

pub use api::{ApiConfig, ApiDisplay, ApiSource, GitRef, MergeRules, Operation, ResolvedSource, operation_label};
pub use asset::{Asset, AssetVariant};
pub use broadcast::{Broadcast, Event, LocalBroadcast};
pub use checks::{Problem, ProblemKind, doc_problems, url_problems};
pub use document::{Document, Mark, Node};
pub use error::CoreError;
pub use find::{excerpt, plain_text};
pub use hotfix::{PullReason, Pulled, hotfix_pages};
pub use id::{AssetId, LockToken, PageId, ReleaseId, RevisionId, UserId};
pub use library::{EntryKind, Library, SNIPPET_NODE, TERM_MARK, Term, VARIABLE_NODE, entry_refs, expand, snippet_cycle, term_anchors, term_refs};
pub use links::{LinkKind, PageLink, extract_links, heading_anchors};
pub use lock::{LOCK_IDLE, LOCK_LEASE, PageLock};
pub use nav::{header_pages, header_section, in_sidebar, sidebar_root};
pub use page::{Page, PageKind, Placement, page_urls, slugify};
pub use redirect::{BrokenLink, Change, ChangeKind, Current, Inactive, Live, LostUrl, OldUrls, Pending, Redirect, RedirectOrigin, effective_redirects, pending_changes, resolve_redirect};
pub use release::{Release, ReleaseStatus};
pub use review::{PageReview, StalePage};
pub use settings::{Banner, Footer, FooterColumn, Mode, ModeColors, Settings, SiteLink, contrast_ratio};
pub use storage::{AssetStore, ContentStore, Draft, Revision, original_key, variant_key};
pub use user::{ApiToken, Role, User};
