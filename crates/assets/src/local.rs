use std::path::Path;

use cairn_core::{AssetStore, CoreError};
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, local::LocalFileSystem, path::Path as StorePath};

/// Assets on the local disk, behind the same `object_store` API an S3 bucket will use.
pub struct LocalAssetStore {
  store: LocalFileSystem,
}

impl LocalAssetStore {
  /// Stores under `dir`, which is created if needed.
  pub fn new(dir: &Path) -> Result<Self, CoreError> {
    std::fs::create_dir_all(dir).map_err(|e| CoreError::Other(e.into()))?;
    let store = LocalFileSystem::new_with_prefix(dir).map_err(|e| CoreError::Other(e.into()))?;

    Ok(Self { store })
  }
}

fn other(error: object_store::Error) -> CoreError {
  match error {
    object_store::Error::NotFound { .. } => CoreError::NotFound,
    error => CoreError::Other(error.into()),
  }
}

impl AssetStore for LocalAssetStore {
  async fn put(&self, key: &str, bytes: Vec<u8>) -> Result<(), CoreError> {
    // Content-addressed keys: the same key always holds the same bytes, so overwriting is harmless.
    self.store.put(&StorePath::from(key), PutPayload::from(bytes)).await.map_err(other)?;

    Ok(())
  }

  async fn get(&self, key: &str) -> Result<Vec<u8>, CoreError> {
    let result = self.store.get(&StorePath::from(key)).await.map_err(other)?;

    Ok(result.bytes().await.map_err(other)?.to_vec())
  }

  async fn exists(&self, key: &str) -> Result<bool, CoreError> {
    match self.store.head(&StorePath::from(key)).await {
      Ok(_) => Ok(true),
      Err(object_store::Error::NotFound { .. }) => Ok(false),
      Err(error) => Err(other(error)),
    }
  }

  async fn delete(&self, key: &str) -> Result<(), CoreError> {
    match self.store.delete(&StorePath::from(key)).await {
      Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
      Err(error) => Err(other(error)),
    }
  }

  async fn list(&self, folder: &str) -> Result<Vec<String>, CoreError> {
    match self.store.list_with_delimiter(Some(&StorePath::from(folder))).await {
      Ok(listed) => Ok(listed.objects.into_iter().map(|o| o.location.to_string()).collect()),
      Err(object_store::Error::NotFound { .. }) => Ok(Vec::new()),
      Err(error) => Err(other(error)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn stores_reads_and_deletes_by_key() {
    let dir = std::env::temp_dir().join(format!("cairn-assets-{}", uuid::Uuid::now_v7()));
    let store = LocalAssetStore::new(&dir).unwrap();

    assert!(!store.exists("originals/abc").await.unwrap());
    store.put("originals/abc", vec![1, 2, 3]).await.unwrap();
    store.put("originals/abc", vec![1, 2, 3]).await.unwrap();

    assert_eq!(store.get("originals/abc").await.unwrap(), [1, 2, 3]);
    assert!(store.exists("originals/abc").await.unwrap());

    store.put("originals/def", vec![4]).await.unwrap();
    let mut listed = store.list("originals").await.unwrap();
    listed.sort();
    assert_eq!(listed, ["originals/abc", "originals/def"]);
    assert!(store.list("renders").await.unwrap().is_empty());

    store.delete("originals/abc").await.unwrap();
    assert!(matches!(store.get("originals/abc").await, Err(CoreError::NotFound)));

    std::fs::remove_dir_all(dir).ok();
  }
}
