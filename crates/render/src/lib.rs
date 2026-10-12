//! Renders documents to HTML in embedded V8.

#![forbid(unsafe_code)]

use ssr_rs::Ssr;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
  #[error("render failed: {0}")]
  Invalid(String),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}

/// Calls `SSR.render(doc)` (ssr_rs takes the exported object as entry point) from a bundled `ssr.js`; one instance per thread because V8 isolates are `!Send`.
pub struct Renderer {
  ssr: Ssr<'static, 'static>,
}

impl Renderer {
  pub fn new(script: &str) -> Result<Self, RenderError> {
    // Safe to call repeatedly: V8 ignores a second platform initialization.
    Ssr::create_platform();

    let ssr = Ssr::from(script.to_string(), "SSR").map_err(|e| RenderError::Invalid(e.to_string()))?;

    Ok(Self { ssr })
  }

  pub fn render(&mut self, doc: &serde_json::Value) -> Result<String, RenderError> {
    let params = serde_json::to_string(doc).map_err(|e| RenderError::Other(e.into()))?;

    self.ssr.render_to_string(Some(&params)).map_err(|e| RenderError::Invalid(e.to_string()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn script() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/renderer/dist/ssr.js");
    std::fs::read_to_string(path).expect("ssr.js is missing, run `bun run build` first")
  }

  #[test]
  fn renders_tab_labels() {
    let doc: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/tabs.json")).unwrap();
    let html = Renderer::new(&script()).unwrap().render(&doc).unwrap();

    for expected in ["Install", "Linux", "macOS", "Use Homebrew."] {
      assert!(html.contains(expected), "missing {expected} in {html}");
    }
  }
}
