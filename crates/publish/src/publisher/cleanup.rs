//! Deleting uploads that nothing can show any more, and stored files that belong to no upload.

use cairn_core::{AssetId, AssetStore, Broadcast, ContentStore, original_key, variant_key};
use cairn_store::{asset_hashes, delete_assets, unused_assets};
use jiff::Timestamp;

use super::Publisher;
use crate::PublishError;

/// What one cleanup deleted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Cleanup {
  pub assets: usize,
  pub files: usize,
}

impl<C: ContentStore, A: AssetStore, B: Broadcast> Publisher<C, A, B> {
  /// Deletes the assets uploaded before `before` that nothing names any more (see `unused_assets`), the row first and then its files, and
  /// any original, variant or unfinished upload written before `before` that no asset owns. Takes the build turn, so no release is
  /// reading these files meanwhile.
  pub async fn clean_assets(&self, before: Timestamp) -> Result<Cleanup, PublishError> {
    let _turn = self.running.lock().await;
    let mut done = Cleanup::default();

    let unused = unused_assets(&self.db, before).await?;
    let ids: Vec<AssetId> = unused.iter().map(|a| a.id).collect();
    // A row never points at a missing file: rows go first, and files left by a failure here are swept as orphans next time.
    delete_assets(&self.db, &ids).await?;
    done.assets = unused.len();
    for asset in &unused {
      let keys = std::iter::once(original_key(&asset.sha256)).chain(asset.variants.iter().map(|v| variant_key(&asset.sha256, v.width)));
      for key in keys {
        self.assets.delete(&key).await?;
        done.files += 1;
      }
    }

    let owned = asset_hashes(&self.db).await?;
    let mut orphans = Vec::new();
    for key in self.assets.list_older("originals", before).await? {
      if !owned.contains(key.rsplit('/').next().unwrap_or_default()) {
        orphans.push(key);
      }
    }
    for key in self.assets.list_older("variants", before).await? {
      let name = key.rsplit('/').next().unwrap_or_default();
      if !owned.contains(name.rsplit_once('-').map_or(name, |(sha256, _)| sha256)) {
        orphans.push(key);
      }
    }
    // An upload is staged for the seconds it takes to arrive; one this old was abandoned.
    orphans.extend(self.assets.list_older("uploads", before).await?);
    for key in orphans {
      self.assets.delete(&key).await?;
      done.files += 1;
    }

    if done != Cleanup::default() {
      tracing::info!(assets = done.assets, files = done.files, "unused uploads deleted");
    }

    Ok(done)
  }
}
