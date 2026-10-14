use std::{
  path::{Component, Path, PathBuf},
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use cairn_core::ReleaseId;

use crate::{CloudflareTarget, PagesTarget, PublishError, S3Target};

/// A file of the published site, with a path relative to the site root.
pub struct OutputFile {
  pub path: String,
  pub bytes: Vec<u8>,
}

/// Counts the files a target sends, for the editor's progress bar. A target calls `start` once it knows how many files it must send (only
/// the ones it lacks, for the targets that compare), then `advance` as they go.
pub struct Progress {
  sent: AtomicUsize,
  total: AtomicUsize,
  report: Box<dyn Fn(usize, usize) + Send + Sync>,
}

impl Progress {
  pub fn new(report: impl Fn(usize, usize) + Send + Sync + 'static) -> Arc<Self> {
    Arc::new(Self {
      sent: AtomicUsize::new(0),
      total: AtomicUsize::new(0),
      report: Box::new(report),
    })
  }

  /// For a build nobody watches.
  pub fn none() -> Arc<Self> {
    Self::new(|_, _| {})
  }

  pub(crate) fn start(&self, total: usize) {
    self.sent.store(0, Ordering::Relaxed);
    self.total.store(total, Ordering::Relaxed);
    (self.report)(0, total);
  }

  pub(crate) fn advance(&self, files: usize) {
    let sent = self.sent.fetch_add(files, Ordering::Relaxed) + files;
    (self.report)(sent, self.total.load(Ordering::Relaxed));
  }
}

/// What the configured target can do; the editor hides what it cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
  /// Whether a build can be deployed for review before going live.
  pub preview: bool,
  /// Whether a build that was looked at can be made live exactly as it is. Without it, publishing builds the site again.
  pub promote: bool,
  /// The most files a release may hold, when the target has a limit.
  pub max_files: Option<usize>,
}

/// What a target answers for a build it took: the version it made of it, and where it can be looked at before it is live.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Deployed {
  pub version: Option<String>,
  /// An address of its own. Targets without one (the self-hosted target shows releases under the editor's address) leave it out.
  pub preview_url: Option<String>,
}

/// Where a release goes. An enum, not a trait object: it is built from the environment at startup, with a `match` in each method.
pub enum PublishTarget {
  /// A folder on this machine that something else serves; the local dev server is one such.
  Folder(FolderTarget),
  /// A folder this server serves itself at the site's address, with releases viewable under `/_preview/<id>/` by signed-in editors.
  SelfHosted(FolderTarget),
  /// A Cloudflare Worker with static assets: builds are uploaded as versions, which can be looked at and then deployed.
  Cloudflare(CloudflareTarget),
  /// An S3 bucket (or compatible storage) with website hosting: files are uploaded as they changed, with no previews.
  S3(S3Target),
  /// A Cloudflare Pages project: previews are deployments of another branch and cannot be promoted.
  Pages(PagesTarget),
}

impl PublishTarget {
  pub fn capabilities(&self) -> Capabilities {
    match self {
      PublishTarget::Folder(_) | PublishTarget::S3(_) => Capabilities {
        preview: false,
        promote: false,
        max_files: None,
      },
      PublishTarget::SelfHosted(_) => Capabilities {
        preview: true,
        promote: true,
        max_files: None,
      },
      PublishTarget::Cloudflare(_) => Capabilities {
        preview: true,
        promote: true,
        max_files: Some(crate::cloudflare::MAX_FILES),
      },
      // A Pages preview is a deployment of another branch, which cannot become the production one.
      PublishTarget::Pages(_) => Capabilities {
        preview: true,
        promote: false,
        max_files: Some(crate::pages::MAX_FILES),
      },
    }
  }

  /// What the editor calls the target.
  pub fn label(&self) -> &'static str {
    match self {
      PublishTarget::Folder(_) => "Local folder",
      PublishTarget::SelfHosted(_) => "This server",
      PublishTarget::Cloudflare(_) => "Cloudflare Workers",
      PublishTarget::Pages(_) => "Cloudflare Pages",
      PublishTarget::S3(_) => "S3 bucket",
    }
  }

  /// What would make the target refuse these files, found before anything is sent.
  pub fn check(&self, files: &[OutputFile]) -> Vec<String> {
    match self {
      PublishTarget::Folder(_) | PublishTarget::SelfHosted(_) => Vec::new(),
      PublishTarget::Cloudflare(_) => CloudflareTarget::check(files),
      PublishTarget::S3(_) => S3Target::check(files),
      PublishTarget::Pages(_) => PagesTarget::check(files),
    }
  }

  /// Hands a release to the target. With `live` it becomes the site; without, it is only put where it can be looked at.
  pub async fn publish(&self, release: ReleaseId, files: Vec<OutputFile>, live: bool, progress: Arc<Progress>) -> Result<Deployed, PublishError> {
    match self {
      PublishTarget::Folder(folder) | PublishTarget::SelfHosted(folder) => {
        if !live && !self.capabilities().preview {
          return Err(PublishError::Other(anyhow::anyhow!("this publish target has no previews")));
        }
        folder.publish(release, files, live, progress).await.map(|()| Deployed::default())
      }
      PublishTarget::Pages(pages) => {
        if live {
          return Ok(Deployed {
            version: Some(pages.publish(&files, &progress).await?),
            preview_url: None,
          });
        }
        let (id, url) = pages.upload(&files, &progress).await?;

        Ok(Deployed { version: Some(id), preview_url: url })
      }
      PublishTarget::S3(s3) => {
        if !live {
          return Err(PublishError::Other(anyhow::anyhow!("this publish target has no previews")));
        }
        s3.publish(&files, &progress).await.map(|()| Deployed::default())
      }
      PublishTarget::Cloudflare(cloudflare) => {
        let version = if live {
          cloudflare.publish(&files, &progress).await?
        } else {
          cloudflare.upload(&files, &progress).await?
        };

        Ok(Deployed {
          version: Some(version.id),
          preview_url: version.preview_url,
        })
      }
    }
  }

  /// Makes a release that was handed over without going live the site, as it was handed over.
  pub async fn promote(&self, release: ReleaseId, version: Option<&str>) -> Result<(), PublishError> {
    match self {
      PublishTarget::Folder(_) | PublishTarget::S3(_) => Err(PublishError::Other(anyhow::anyhow!("this publish target has no previews"))),
      PublishTarget::Pages(_) => Err(PublishError::Other(anyhow::anyhow!("Cloudflare Pages cannot make a preview the live site; publish to deploy it"))),
      PublishTarget::SelfHosted(folder) => folder.switch(release).await,
      PublishTarget::Cloudflare(cloudflare) => {
        cloudflare
          .deploy(version.ok_or_else(|| PublishError::Other(anyhow::anyhow!("the release has no Cloudflare version")))?)
          .await
      }
    }
  }

  /// The folder of the live site, when the files are on this machine.
  pub fn site_dir(&self) -> Option<PathBuf> {
    match self {
      PublishTarget::Folder(folder) | PublishTarget::SelfHosted(folder) => Some(folder.site_dir()),
      PublishTarget::Cloudflare(_) | PublishTarget::S3(_) | PublishTarget::Pages(_) => None,
    }
  }

  /// The folder holding every kept release, one directory per release ID, for the targets that can show them.
  pub fn releases_dir(&self) -> Option<PathBuf> {
    match self {
      PublishTarget::SelfHosted(folder) => Some(folder.releases_dir()),
      PublishTarget::Folder(_) | PublishTarget::Cloudflare(_) | PublishTarget::S3(_) | PublishTarget::Pages(_) => None,
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

  pub fn releases_dir(&self) -> PathBuf {
    self.root.join("releases")
  }

  async fn publish(&self, release: ReleaseId, files: Vec<OutputFile>, live: bool, progress: Arc<Progress>) -> Result<(), PublishError> {
    let root = self.root.clone();

    tokio::task::spawn_blocking(move || {
      write_release(&root, release, files, &progress)?;
      if live {
        switch_site(&root, release)?;
      }
      prune(&root.join("releases"), &root.join("site"));

      Ok::<_, std::io::Error>(())
    })
    .await
    .map_err(|e| PublishError::Other(e.into()))?
    .map_err(|e| PublishError::Other(e.into()))
  }

  /// Makes a release that is already on disk the live site.
  async fn switch(&self, release: ReleaseId) -> Result<(), PublishError> {
    let root = self.root.clone();

    tokio::task::spawn_blocking(move || {
      if !root.join("releases").join(release.to_string()).is_dir() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the release is no longer on disk"));
      }
      switch_site(&root, release)
    })
    .await
    .map_err(|e| PublishError::Other(e.into()))?
    .map_err(|e| PublishError::Other(e.into()))
  }
}

fn write_release(root: &Path, release: ReleaseId, files: Vec<OutputFile>, progress: &Progress) -> std::io::Result<()> {
  let releases = root.join("releases");
  let partial = releases.join(format!("{release}.partial"));
  let done = releases.join(release.to_string());

  std::fs::create_dir_all(&partial)?;
  progress.start(files.len());

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
    progress.advance(1);
  }

  std::fs::rename(&partial, &done)
}

fn switch_site(root: &Path, release: ReleaseId) -> std::io::Result<()> {
  let link = root.join("site");
  let staging = root.join(".site-next");
  // A leftover from a crashed publish.
  let _ = std::fs::remove_file(&staging);
  std::os::unix::fs::symlink(Path::new("releases").join(release.to_string()), &staging)?;
  std::fs::rename(&staging, &link)
}

/// Removes old releases, keeping the newest few (IDs are time-ordered) and always the live one, which `site` points to.
fn prune(releases: &Path, site: &Path) {
  let Ok(entries) = std::fs::read_dir(releases) else { return };
  let live = std::fs::read_link(site).ok().and_then(|target| target.file_name().map(|name| releases.join(name)));
  let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
  dirs.sort();

  let excess = dirs.len().saturating_sub(KEEP_RELEASES + 1);
  for dir in dirs.into_iter().take(excess).filter(|d| Some(d) != live.as_ref()) {
    let _ = std::fs::remove_dir_all(dir);
  }
}
