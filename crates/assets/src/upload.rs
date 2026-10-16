use bytes::Bytes;
use cairn_core::CoreError;
use futures_util::{Stream, StreamExt};
use object_store::{ObjectStoreExt, WriteMultipart, path::Path as StorePath};
use sha2::{Digest, Sha256};

use crate::{ImageInfo, ObjectAssetStore, check_size, inspect, store::other};

/// How much of the start of a file is kept to read its type and size: JPEG puts its size after the EXIF block, which can be large.
const HEAD_MAX: usize = 1 << 20;

/// Parts sent to the store at once while an upload streams in; with 5 MiB parts this bounds the memory one upload holds.
const PARTS_IN_FLIGHT: usize = 2;

/// An upload written to a temporary key, waiting to be kept under its content hash or discarded.
#[derive(Debug)]
pub struct Staged {
  key: String,
  pub sha256: String,
  pub bytes: u64,
  pub info: ImageInfo,
}

impl ObjectAssetStore {
  /// Streams an upload into the store without holding it in memory, hashing it on the way and reading its type and size from its
  /// first bytes. A file that is not an accepted image, has too many pixels or passes `limit` bytes is refused and nothing is left behind.
  pub async fn stage<S, E>(&self, body: S, limit: u64) -> Result<Staged, CoreError>
  where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
  {
    let key = format!("uploads/{}", uuid::Uuid::now_v7());
    let upload = self.store.put_multipart(&StorePath::from(key.as_str())).await.map_err(other)?;
    let mut writer = WriteMultipart::new(upload);

    match receive(body, limit, &mut writer).await {
      Ok((sha256, bytes, info)) => {
        writer.finish().await.map_err(other)?;
        Ok(Staged { key, sha256, bytes, info })
      }
      Err(error) => {
        writer.abort().await.ok();
        Err(error)
      }
    }
  }

  /// The staged file's bytes, to make its variants from.
  pub async fn read_staged(&self, staged: &Staged) -> Result<Vec<u8>, CoreError> {
    let result = self.store.get(&StorePath::from(staged.key.as_str())).await.map_err(other)?;

    Ok(result.bytes().await.map_err(other)?.to_vec())
  }

  /// Moves the staged file to `key`. Keys are content hashes, so replacing a file already there changes nothing.
  pub async fn keep(&self, staged: Staged, key: &str) -> Result<(), CoreError> {
    self.store.rename(&StorePath::from(staged.key.as_str()), &StorePath::from(key)).await.map_err(other)
  }

  pub async fn discard(&self, staged: Staged) -> Result<(), CoreError> {
    match self.store.delete(&StorePath::from(staged.key.as_str())).await {
      Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
      Err(error) => Err(other(error)),
    }
  }
}

async fn receive<S, E>(mut body: S, limit: u64, writer: &mut WriteMultipart) -> Result<(String, u64, ImageInfo), CoreError>
where
  S: Stream<Item = Result<Bytes, E>> + Unpin,
  E: std::error::Error + Send + Sync + 'static,
{
  let mut hasher = Sha256::new();
  let mut bytes = 0u64;
  let mut head = Vec::new();
  let mut info = None;

  while let Some(chunk) = body.next().await {
    let chunk = chunk.map_err(|e| CoreError::Other(e.into()))?;
    bytes += chunk.len() as u64;
    if bytes > limit {
      return Err(CoreError::TooLarge { limit });
    }

    if info.is_none() {
      head.extend_from_slice(&chunk[..chunk.len().min(HEAD_MAX - head.len())]);
      // A refusal is only final once the header had every chance to arrive.
      match inspect(&head) {
        Ok(found) => {
          check_size(&found)?;
          info = Some(found);
          head = Vec::new();
        }
        Err(error) if head.len() >= HEAD_MAX => return Err(error),
        Err(_) => {}
      }
    }

    hasher.update(&chunk);
    writer.wait_for_capacity(PARTS_IN_FLIGHT).await.map_err(other)?;
    writer.put(chunk);
  }

  let info = match info {
    Some(info) => info,
    None => {
      let info = inspect(&head)?;
      check_size(&info)?;
      info
    }
  };
  let sha256 = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();

  Ok((sha256, bytes, info))
}
