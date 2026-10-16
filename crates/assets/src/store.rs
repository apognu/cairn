use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use cairn_core::{AssetStore, CoreError};
use object_store::{
  ClientOptions, ObjectStore, ObjectStoreExt, PutPayload, aws::AmazonS3Builder, gcp::GoogleCloudStorageBuilder, local::LocalFileSystem, path::Path as StorePath, prefix::PrefixStore,
};

/// Where the asset store keeps its files, chosen with `ASSET_STORE` at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetBackend {
  /// A folder on the local disk.
  Local(PathBuf),
  /// An S3 bucket, or a compatible one (R2, MinIO).
  S3(S3Settings),
  /// A Google Cloud Storage bucket.
  Gcs(GcsSettings),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Settings {
  pub bucket: String,
  pub region: String,
  /// `None` for AWS itself; R2 and MinIO give theirs.
  pub endpoint: Option<String>,
  /// Without keys the usual AWS sources are used (environment, instance or task role).
  pub access_key: Option<String>,
  pub secret_key: Option<String>,
  /// Where in the bucket the files go, without slashes at the ends; empty for the root.
  pub prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcsSettings {
  pub bucket: String,
  /// The path of a service account key file; without it the container's own identity is used (Cloud Run, GKE).
  pub credentials: Option<PathBuf>,
  /// An emulator's address (`http://localhost:4443`); `None` for Google itself.
  pub endpoint: Option<String>,
  pub prefix: String,
}

/// Assets in a folder or a bucket, through `object_store`, keyed by content hash.
pub struct ObjectAssetStore {
  store: Arc<dyn ObjectStore>,
}

impl ObjectAssetStore {
  pub fn open(backend: &AssetBackend) -> Result<Self, CoreError> {
    // object_store's HTTP client is built without a TLS crypto provider (to stay off aws-lc-rs), so the process needs one installed;
    // an error only means another part of the process installed it first.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let store: Arc<dyn ObjectStore> = match backend {
      AssetBackend::Local(dir) => return Self::local(dir),
      AssetBackend::S3(s3) => {
        let mut builder = AmazonS3Builder::from_env().with_bucket_name(&s3.bucket).with_region(&s3.region);
        if let Some(endpoint) = &s3.endpoint {
          builder = builder.with_endpoint(endpoint).with_allow_http(endpoint.starts_with("http://"));
        }
        if let (Some(key), Some(secret)) = (&s3.access_key, &s3.secret_key) {
          builder = builder.with_access_key_id(key).with_secret_access_key(secret);
        }

        prefixed(Arc::new(builder.build().context("the S3 asset store is not usable")?), &s3.prefix)
      }
      AssetBackend::Gcs(gcs) => {
        let mut builder = GoogleCloudStorageBuilder::from_env().with_bucket_name(&gcs.bucket);
        if let Some(path) = &gcs.credentials {
          builder = builder.with_service_account_path(path.to_string_lossy());
        }
        if let Some(endpoint) = &gcs.endpoint {
          builder = builder
            .with_base_url(endpoint)
            .with_client_options(ClientOptions::new().with_allow_http(endpoint.starts_with("http://")))
            // An emulator takes unsigned requests, and there is no identity to sign with off Google Cloud.
            .with_skip_signature(gcs.credentials.is_none());
        }

        prefixed(Arc::new(builder.build().context("the GCS asset store is not usable")?), &gcs.prefix)
      }
    };

    Ok(Self { store })
  }

  /// Stores under `dir`, which is created if needed.
  pub fn local(dir: &std::path::Path) -> Result<Self, CoreError> {
    std::fs::create_dir_all(dir).map_err(|e| CoreError::Other(e.into()))?;
    let store = LocalFileSystem::new_with_prefix(dir).map_err(|e| CoreError::Other(e.into()))?;

    Ok(Self { store: Arc::new(store) })
  }
}

fn prefixed(store: Arc<dyn ObjectStore>, prefix: &str) -> Arc<dyn ObjectStore> {
  match prefix.trim_matches('/') {
    "" => store,
    prefix => Arc::new(PrefixStore::new(store, prefix)),
  }
}

fn other(error: object_store::Error) -> CoreError {
  match error {
    object_store::Error::NotFound { .. } => CoreError::NotFound,
    error => CoreError::Other(error.into()),
  }
}

impl AssetStore for ObjectAssetStore {
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
    let store = ObjectAssetStore::local(&dir).unwrap();

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
