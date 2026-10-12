use serde::{Deserialize, Serialize};

use crate::AssetId;

/// The site-wide settings of the theme basics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Settings {
  pub site_name: String,
  /// An uploaded image shown before the site name in the header.
  pub logo_asset_id: Option<AssetId>,
  /// `#rrggbb`.
  pub accent: String,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      site_name: "Documentation".into(),
      logo_asset_id: None,
      accent: "#0b6e4f".into(),
    }
  }
}

impl Settings {
  /// Checks the values a user can type; the accent ends up in a stylesheet, so it must be exactly a hex color.
  pub fn validate(&self) -> Result<(), String> {
    let name = self.site_name.trim();
    if name.is_empty() || name.chars().count() > 80 {
      return Err("the site name must be between 1 and 80 characters".into());
    }

    let hex = self.accent.strip_prefix('#').unwrap_or_default();
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
      return Err("the accent color must look like #0b6e4f".into());
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validates_name_and_accent() {
    let ok = Settings::default();
    assert!(ok.validate().is_ok());

    for bad in [
      Settings { site_name: "  ".into(), ..ok.clone() },
      Settings { accent: "red".into(), ..ok.clone() },
      Settings {
        accent: "#12345".into(),
        ..ok.clone()
      },
      Settings {
        accent: "#0b6e4f; } body { display: none".into(),
        ..ok.clone()
      },
    ] {
      assert!(bad.validate().is_err(), "{bad:?}");
    }
  }
}
