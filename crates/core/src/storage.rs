use std::future::Future;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{CoreError, Document, PageId, RevisionId, UserId};

/// The latest saved edits of a page; there is one per page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
  pub page_id: PageId,
  pub doc: Document,
  pub base_revision_id: Option<RevisionId>,
  pub updated_by: UserId,
  pub updated_at: Timestamp,
}

/// An immutable snapshot of a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Revision {
  pub id: RevisionId,
  pub page_id: PageId,
  pub doc: Document,
  pub title: String,
  /// The page's description when this version was saved.
  pub description: Option<String>,
  /// False for versions saved before descriptions were recorded: `description` says nothing then.
  pub description_recorded: bool,
  pub author_id: UserId,
  pub created_at: Timestamp,
}

/// Where page content lives. The SQL implementation is in `store`; a git-backed one may follow.
pub trait ContentStore: Send + Sync {
  fn draft(&self, page: PageId) -> impl Future<Output = Result<Option<Draft>, CoreError>> + Send;

  /// Saves a draft built on `base`. Fails with `CoreError::Conflict` when `base` is not the page's latest revision.
  fn save_draft(&self, page: PageId, doc: Document, base: Option<RevisionId>, user: UserId) -> impl Future<Output = Result<Draft, CoreError>> + Send;

  fn revision(&self, id: RevisionId) -> impl Future<Output = Result<Revision, CoreError>> + Send;

  fn latest_revision(&self, page: PageId) -> impl Future<Output = Result<Option<Revision>, CoreError>> + Send;

  /// The page's latest revision, creating an empty one if it has none. Unlike a save, this leaves the draft alone, so publishing a
  /// page nobody has edited cannot make an editor's first save look stale.
  fn ensure_revision(&self, page: PageId, user: UserId) -> impl Future<Output = Result<Revision, CoreError>> + Send;
}

/// Where uploaded files and their variants live. Bytes are held in memory for now; streaming comes with large uploads.
pub trait AssetStore: Send + Sync {
  /// Stores `bytes` under `key` (a content-hash path). Storing the same key twice is a no-op.
  fn put(&self, key: &str, bytes: Vec<u8>) -> impl Future<Output = Result<(), CoreError>> + Send;

  fn get(&self, key: &str) -> impl Future<Output = Result<Vec<u8>, CoreError>> + Send;

  fn exists(&self, key: &str) -> impl Future<Output = Result<bool, CoreError>> + Send;

  fn delete(&self, key: &str) -> impl Future<Output = Result<(), CoreError>> + Send;

  /// The keys of the files directly inside `folder` (`renders`), as `put` took them; none when the folder does not exist.
  fn list(&self, folder: &str) -> impl Future<Output = Result<Vec<String>, CoreError>> + Send;

  /// Like `list`, only the files last written before `before`.
  fn list_older(&self, folder: &str, before: jiff::Timestamp) -> impl Future<Output = Result<Vec<String>, CoreError>> + Send;
}

/// Where the WebP copy of an image, `width` pixels wide, lives in the `AssetStore`.
pub fn variant_key(sha256: &str, width: u32) -> String {
  format!("variants/{sha256}-{width}.webp")
}

/// Where the original file of an asset lives in the `AssetStore`.
pub fn original_key(sha256: &str) -> String {
  format!("originals/{sha256}")
}
