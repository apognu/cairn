use std::path::{Component, Path, PathBuf};

use yourdocs_core::ReleaseId;

use crate::PublishError;

/// A file of the published site, with a path relative to the site root.
pub struct OutputFile {
  pub path: String,
  pub bytes: Vec<u8>,
}

/// What the configured target can do; the editor hides what it cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
  /// Whether a build can be deployed for review before going live.
  pub preview: bool,
}

/// Where a release goes. An enum, not a trait object: it is built from the environment at startup, with a `match` in each method.
pub enum PublishTarget {
  Folder(FolderTarget),
}

impl PublishTarget {
  pub fn capabilities(&self) -> Capabilities {
    match self {
      PublishTarget::Folder(_) => Capabilities { preview: false },
    }
  }

  pub async fn publish(&self, release: ReleaseId, files: Vec<OutputFile>) -> Result<(), PublishError> {
    match self {
      PublishTarget::Folder(folder) => folder.publish(release, files).await,
    }
  }
}

/// Writes releases under `<root>/releases/<id>` and makes `<root>/site` a symlink to the live one.
///
/// Switching is one `rename` of a new symlink over the old, so a reader sees the old site or the new one, never a mix; and the new
/// release is written completely before the switch, so a failure leaves the live site as it was.
pub struct FolderTarget {
  root: PathBuf,
}

/// Releases kept on disk besides the live one, for rollback.
const KEEP_RELEASES: usize = 5;

impl FolderTarget {
  pub fn new(root: PathBuf) -> Self {
    Self { root }
  }

  /// The directory a server should serve.
  pub fn site_dir(&self) -> PathBuf {
    self.root.join("site")
  }

  async fn publish(&self, release: ReleaseId, files: Vec<OutputFile>) -> Result<(), PublishError> {
    let root = self.root.clone();

    tokio::task::spawn_blocking(move || write_release(&root, release, files))
      .await
      .map_err(|e| PublishError::Other(e.into()))?
      .map_err(|e| PublishError::Other(e.into()))
  }
}

fn write_release(root: &Path, release: ReleaseId, files: Vec<OutputFile>) -> std::io::Result<()> {
  let releases = root.join("releases");
  let partial = releases.join(format!("{release}.partial"));
  let done = releases.join(release.to_string());

  std::fs::create_dir_all(&partial)?;

  for file in files {
    let relative = Path::new(&file.path);
    // Paths are built from slugs and IDs, but a file must never land outside its release.
    if relative.components().any(|c| !matches!(c, Component::Normal(_))) {
      return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("unsafe output path {}", file.path)));
    }

    let target = partial.join(relative);
    if let Some(parent) = target.parent() {
      std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, file.bytes)?;
  }

  std::fs::rename(&partial, &done)?;

  let link = root.join("site");
  let staging = root.join(".site-next");
  // A leftover from a crashed publish.
  let _ = std::fs::remove_file(&staging);
  std::os::unix::fs::symlink(Path::new("releases").join(release.to_string()), &staging)?;
  std::fs::rename(&staging, &link)?;

  prune(&releases, &done);
  Ok(())
}

/// Removes old releases, keeping the newest few (IDs are time-ordered) and always the live one.
fn prune(releases: &Path, live: &Path) {
  let Ok(entries) = std::fs::read_dir(releases) else { return };
  let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
  dirs.sort();

  let excess = dirs.len().saturating_sub(KEEP_RELEASES + 1);
  for dir in dirs.into_iter().take(excess).filter(|d| d != live) {
    let _ = std::fs::remove_dir_all(dir);
  }
}
