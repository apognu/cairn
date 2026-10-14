use std::path::Path;

use cairn_core::ReleaseId;
use tokio::process::Command;

use crate::{PublishError, target::OutputFile};

/// Builds the Pagefind search index for the HTML files of a release and returns the files it produced (under `pagefind/`).
///
/// Returns `None`, with a warning, when the Pagefind binary is not installed: the site is still published, just without search.
pub(crate) async fn build_index(binary: &str, release: ReleaseId, html: &[&OutputFile]) -> Result<Option<Vec<OutputFile>>, PublishError> {
  let dir = std::env::temp_dir().join(format!("cairn-search-{release}"));
  let result = run(binary, &dir, html).await;
  let _ = tokio::fs::remove_dir_all(&dir).await;

  result
}

async fn run(binary: &str, dir: &Path, html: &[&OutputFile]) -> Result<Option<Vec<OutputFile>>, PublishError> {
  for file in html {
    let path = dir.join(&file.path);
    if let Some(parent) = path.parent() {
      tokio::fs::create_dir_all(parent).await.map_err(|e| PublishError::Other(e.into()))?;
    }
    tokio::fs::write(path, &file.bytes).await.map_err(|e| PublishError::Other(e.into()))?;
  }

  let output = match Command::new(binary).arg("--site").arg(dir).output().await {
    Ok(output) => output,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
      tracing::warn!(binary, "pagefind is not installed: publishing without search");
      return Ok(None);
    }
    Err(error) => return Err(PublishError::Other(error.into())),
  };

  if !output.status.success() {
    return Err(PublishError::Other(anyhow::anyhow!("pagefind failed: {}", String::from_utf8_lossy(&output.stderr))));
  }

  let mut files = Vec::new();
  let mut stack = vec![dir.join("pagefind")];

  while let Some(current) = stack.pop() {
    let mut entries = tokio::fs::read_dir(&current).await.map_err(|e| PublishError::Other(e.into()))?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| PublishError::Other(e.into()))? {
      let path = entry.path();
      if path.is_dir() {
        stack.push(path);
      } else {
        let relative = path.strip_prefix(dir).map_err(|e| PublishError::Other(e.into()))?;
        let bytes = tokio::fs::read(&path).await.map_err(|e| PublishError::Other(e.into()))?;
        files.push(OutputFile {
          path: relative.to_string_lossy().into_owned(),
          bytes,
        });
      }
    }
  }

  Ok(Some(files))
}
