use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::assets::Assets;

/// The component SDK version this server renders with. `packages/sdk` has the same number; a test keeps them equal.
pub(crate) const SDK_VERSION: u64 = 1;

/// The JavaScript side of the site: what V8 renders with, what the browser loads, and the editor. Either the ones embedded in the binary or
/// the ones `build-components` made (which add the deployer's components).
pub(crate) struct BundleSet {
  pub(crate) ssr_js: String,
  pub(crate) client_js: Vec<u8>,
  pub(crate) site_css: Vec<u8>,
  /// The API reference script, written into releases that have API pages and served to the editor's preview.
  pub(crate) scalar_js: Vec<u8>,
  /// The editor built with the custom components, served from disk instead of the embedded one.
  pub(crate) editor_dir: Option<PathBuf>,
}

impl BundleSet {
  /// Loads the bundles in `dir` when it has a `manifest.json`, the embedded ones otherwise. A directory that was asked for by name must
  /// have them, and bundles built with another SDK version are refused: their components would render with a different contract.
  pub(crate) fn load(dir: Option<&Path>, required: bool) -> anyhow::Result<Self> {
    let Some(dir) = dir.filter(|dir| dir.join("manifest.json").is_file()) else {
      if let (Some(dir), true) = (dir, required) {
        bail!("{} has no manifest.json; build it with `yourdocs build-components`", dir.display());
      }

      return Self::embedded();
    };

    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).context("cannot read manifest.json")?).context("manifest.json is not valid JSON")?;
    let built_with = manifest["sdk_version"].as_u64().context("manifest.json has no sdk_version")?;
    if built_with != SDK_VERSION {
      bail!(
        "the bundles in {} were built with component SDK version {built_with}, but this server runs SDK version {SDK_VERSION}; rebuild them with the matching builder image",
        dir.display()
      );
    }

    let read = |name: &str| std::fs::read(dir.join(name)).with_context(|| format!("cannot read {} in {}", name, dir.display()));
    let editor = dir.join("editor");

    Ok(Self {
      ssr_js: String::from_utf8(read("ssr.js")?).context("ssr.js is not valid UTF-8")?,
      client_js: read("client.js")?,
      site_css: read("site.css")?,
      scalar_js: read("scalar.js")?,
      editor_dir: editor.join("index.html").is_file().then_some(editor),
    })
  }

  fn embedded() -> anyhow::Result<Self> {
    let embedded = |name: &str| {
      Assets::get(name)
        .map(|f| f.data.into_owned())
        .with_context(|| format!("{name} is not embedded, run `bun run build` before `cargo build`"))
    };

    Ok(Self {
      ssr_js: String::from_utf8(embedded("ssr.js")?).context("ssr.js is not valid UTF-8")?,
      client_js: embedded("client.js")?,
      site_css: embedded("site.css")?,
      scalar_js: embedded("scalar.js")?,
      editor_dir: None,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn bundle_dir(sdk_version: u64) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yourdocs-bundles-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(dir.join("editor")).unwrap();
    std::fs::write(dir.join("manifest.json"), format!(r#"{{ "sdk_version": {sdk_version}, "components": [], "slots": [] }}"#)).unwrap();
    for (name, body) in [
      ("ssr.js", "var SSR = {}"),
      ("client.js", "1"),
      ("site.css", "a{}"),
      ("scalar.js", "2"),
      ("editor/index.html", "<html></html>"),
    ] {
      std::fs::write(dir.join(name), body).unwrap();
    }

    dir
  }

  #[test]
  fn loads_matching_bundles_and_refuses_another_sdk_version() {
    let dir = bundle_dir(SDK_VERSION);
    let set = BundleSet::load(Some(&dir), true).unwrap();
    assert_eq!(set.ssr_js, "var SSR = {}");
    assert_eq!(set.editor_dir, Some(dir.join("editor")));

    let error = BundleSet::load(Some(&bundle_dir(SDK_VERSION + 1)), false).err().expect("must refuse").to_string();
    assert!(error.contains(&format!("SDK version {}", SDK_VERSION + 1)) && error.contains("rebuild"), "{error}");
  }

  #[test]
  fn falls_back_to_the_embedded_bundles_unless_the_directory_was_asked_for() {
    let missing = std::env::temp_dir().join("yourdocs-no-such-bundles");

    assert!(BundleSet::load(Some(&missing), false).unwrap().editor_dir.is_none());
    assert!(BundleSet::load(None, false).is_ok());
    assert!(BundleSet::load(Some(&missing), true).err().unwrap().to_string().contains("no manifest.json"));
  }

  #[test]
  fn the_sdk_version_matches_the_one_the_bundles_are_stamped_with() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/sdk/src/index.ts")).unwrap();

    assert!(source.contains(&format!("export const SDK_VERSION = {SDK_VERSION};")));
  }
}
