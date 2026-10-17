use serde::{Deserialize, Serialize};

use crate::AssetId;

/// The site-wide settings: name, logo, colors and the links around the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct Settings {
  pub site_name: String,
  /// An uploaded image shown before the site name in the header.
  pub logo_asset_id: Option<AssetId>,
  /// Shown instead of the logo in dark mode.
  pub logo_dark_asset_id: Option<AssetId>,
  /// `#rrggbb`.
  pub accent: String,
  pub light: ModeColors,
  pub dark: ModeColors,
  /// External links shown in the header, after the sections.
  pub header_links: Vec<SiteLink>,
  pub footer: Footer,
  pub banner: Banner,
  /// A stylesheet written after the theme, on the published site and in the editor.
  pub custom_css: String,
}

/// Optional color overrides for one mode, each `#rrggbb`. What is not set keeps the built-in color.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct ModeColors {
  pub background: Option<String>,
  pub text: Option<String>,
  pub surface: Option<String>,
  pub link: Option<String>,
  pub code_background: Option<String>,
}

/// A message at the top of every page, which readers can dismiss. Off while `text` is empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct Banner {
  pub text: String,
  /// Where the message leads: `https://…`, `mailto:…` or a path on the site; empty for none.
  pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SiteLink {
  /// Required; also the accessible name when only the icon shows.
  pub label: String,
  /// `https://…`, `mailto:…` or a path on the site.
  pub url: String,
  /// A Lucide icon name. Header links with one show only the icon on wide screens; footer links ignore it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct Footer {
  pub text: String,
  pub columns: Vec<FooterColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FooterColumn {
  pub title: String,
  pub links: Vec<SiteLink>,
}

/// Light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  Light,
  Dark,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      site_name: "Documentation".into(),
      logo_asset_id: None,
      logo_dark_asset_id: None,
      accent: "#0b6e4f".into(),
      light: ModeColors::default(),
      dark: ModeColors::default(),
      header_links: Vec::new(),
      footer: Footer::default(),
      banner: Banner::default(),
      custom_css: String::new(),
    }
  }
}

// The built-in palette of `site.css`; the contrast check needs the colors a reader gets when nothing is overridden.
const LIGHT_BACKGROUND: &str = "#ffffff";
const LIGHT_TEXT: &str = "#16181a";
const LIGHT_SURFACE: &str = "#fafaf9";
const DARK_BACKGROUND: &str = "#121416";
const DARK_TEXT: &str = "#eceeef";
const DARK_SURFACE: &str = "#17191c";

const MAX_LINKS: usize = 12;
const MAX_COLUMNS: usize = 6;
const MAX_CSS: usize = 50_000;

fn is_hex(color: &str) -> bool {
  color.strip_prefix('#').is_some_and(|hex| hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
}

fn rgb(color: &str) -> [f64; 3] {
  let channel = |i: usize| f64::from(u8::from_str_radix(&color[1 + 2 * i..3 + 2 * i], 16).unwrap_or(0));
  [channel(0), channel(1), channel(2)]
}

fn luminance([r, g, b]: [f64; 3]) -> f64 {
  let linear = |c: f64| {
    let c = c / 255.0;
    if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
  };

  0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/// The WCAG contrast ratio of two `#rrggbb` colors, from 1 to 21.
pub fn contrast_ratio(a: &str, b: &str) -> f64 {
  let (a, b) = (luminance(rgb(a)), luminance(rgb(b)));
  let (light, dark) = if a > b { (a, b) } else { (b, a) };

  (light + 0.05) / (dark + 0.05)
}

/// What `site.css` does to the accent in dark mode (`color-mix(in srgb, accent 55%, white)`), so the check sees the color readers get.
fn dark_accent(accent: &str) -> String {
  let [r, g, b] = rgb(accent).map(|c| (c * 0.55 + 255.0 * 0.45).round() as u8);

  format!("#{r:02x}{g:02x}{b:02x}")
}

fn valid_url(url: &str) -> bool {
  let ok_scheme = url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") || (url.starts_with('/') && !url.starts_with("//"));

  ok_scheme && !url.chars().any(|c| c.is_control() || c.is_whitespace()) && url.len() <= 500
}

fn check_links(what: &str, links: &[SiteLink]) -> Result<(), String> {
  if links.len() > MAX_LINKS {
    return Err(format!("{what} can have at most {MAX_LINKS} links"));
  }

  for link in links {
    let label = link.label.trim();
    if label.is_empty() || label.chars().count() > 60 {
      return Err(format!("every {what} link needs a label of up to 60 characters"));
    }
    if !valid_url(&link.url) {
      return Err(format!("the {what} link \"{label}\" must start with https://, http://, mailto: or /"));
    }
    if let Some(icon) = &link.icon
      && (icon.is_empty() || icon.len() > 64 || !icon.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
    {
      return Err(format!("the {what} link \"{label}\" has an icon name that is not one"));
    }
  }

  Ok(())
}

impl Settings {
  /// The colors that end up on screen in one mode, overrides applied: background, text, surface, link.
  fn effective(&self, mode: Mode) -> [String; 4] {
    let (colors, background, text, surface) = match mode {
      Mode::Light => (&self.light, LIGHT_BACKGROUND, LIGHT_TEXT, LIGHT_SURFACE),
      Mode::Dark => (&self.dark, DARK_BACKGROUND, DARK_TEXT, DARK_SURFACE),
    };
    let accent = match mode {
      Mode::Light => self.accent.to_lowercase(),
      Mode::Dark => dark_accent(&self.accent),
    };
    let pick = |value: &Option<String>, default: &str| value.as_deref().unwrap_or(default).to_lowercase();

    [
      pick(&colors.background, background),
      pick(&colors.text, text),
      pick(&colors.surface, surface),
      pick(&colors.link, &accent),
    ]
  }

  /// Text pairs that fall under WCAG AA (4.5:1), worded for the person editing the palette. Needs valid colors.
  pub fn contrast_problems(&self) -> Vec<String> {
    let mut problems = Vec::new();

    for (mode, name) in [(Mode::Light, "light"), (Mode::Dark, "dark")] {
      let [background, text, surface, link] = self.effective(mode);

      for (pair, a, b) in [
        ("Text on the background", &text, &background),
        ("Text on surfaces", &text, &surface),
        ("Links on the background", &link, &background),
        ("Links on surfaces", &link, &surface),
      ] {
        let ratio = contrast_ratio(a, b);
        if ratio < 4.5 {
          problems.push(format!("{pair} in {name} mode has a contrast of {ratio:.1}:1; WCAG AA needs 4.5:1"));
        }
      }
    }

    problems
  }

  /// Checks the values a user can type; colors and links end up in a stylesheet or in HTML, so they must have exactly the expected shape.
  /// A palette that fails WCAG AA is refused too.
  pub fn validate(&self) -> Result<(), String> {
    self.validate_shape()?;

    match self.contrast_problems().into_iter().next() {
      Some(problem) => Err(problem),
      None => Ok(()),
    }
  }

  /// Everything `validate` checks except the contrast, so the editor can show warnings for a palette that is well formed.
  pub fn validate_shape(&self) -> Result<(), String> {
    let name = self.site_name.trim();
    if name.is_empty() || name.chars().count() > 80 {
      return Err("the site name must be between 1 and 80 characters".into());
    }

    if !is_hex(&self.accent) {
      return Err("the accent color must look like #0b6e4f".into());
    }

    for (mode, colors) in [("light", &self.light), ("dark", &self.dark)] {
      for (label, color) in [
        ("background", &colors.background),
        ("text", &colors.text),
        ("surface", &colors.surface),
        ("link", &colors.link),
        ("code background", &colors.code_background),
      ] {
        if let Some(color) = color
          && !is_hex(color)
        {
          return Err(format!("the {mode} {label} color must look like #0b6e4f"));
        }
      }
    }

    check_links("header", &self.header_links)?;

    if self.custom_css.len() > MAX_CSS {
      return Err(format!("the custom CSS can have at most {MAX_CSS} bytes"));
    }
    // The stylesheet is written inside a `<style>` element, which only `</style` can end.
    if self.custom_css.to_ascii_lowercase().contains("</style") {
      return Err("the custom CSS cannot contain </style".into());
    }

    if self.banner.text.chars().count() > 200 {
      return Err("the announcement can have at most 200 characters".into());
    }
    if !self.banner.url.is_empty() && !valid_url(&self.banner.url) {
      return Err("the announcement link must start with https://, http://, mailto: or /".into());
    }

    if self.footer.text.chars().count() > 300 {
      return Err("the footer text can have at most 300 characters".into());
    }
    if self.footer.columns.len() > MAX_COLUMNS {
      return Err(format!("the footer can have at most {MAX_COLUMNS} columns"));
    }
    for column in &self.footer.columns {
      if column.title.trim().is_empty() || column.title.chars().count() > 60 {
        return Err("every footer column needs a title of up to 60 characters".into());
      }
      check_links("footer", &column.links)?;
    }

    Ok(())
  }

  /// The CSS custom properties this mode overrides, in `site.css` terms. The accent is shared; dark mode derives its shade in the stylesheet.
  pub fn theme_vars(&self, mode: Mode) -> Vec<(&'static str, String)> {
    let colors = match mode {
      Mode::Light => &self.light,
      Mode::Dark => &self.dark,
    };
    let mut vars = Vec::new();
    let mut add = |names: &[&'static str], value: &Option<String>| {
      if let Some(value) = value {
        vars.extend(names.iter().map(|name| (*name, value.to_lowercase())));
      }
    };

    add(&["--bg"], &colors.background);
    add(&["--text", "--text-body"], &colors.text);
    add(&["--bg-side"], &colors.surface);
    add(&["--link"], &colors.link);
    add(&["--code-bg"], &colors.code_background);

    vars
  }

  /// The stylesheet that applies the theme and the custom CSS, written after `site.css`. The same rules the stylesheet uses to pick a mode: the reader's
  /// toggle wins, otherwise the system setting.
  pub fn theme_css(&self) -> String {
    fn block(selector: &str, vars: &[(&str, String)]) -> String {
      if vars.is_empty() {
        return String::new();
      }

      format!("{selector}{{{}}}", vars.iter().map(|(name, value)| format!("{name}:{value}")).collect::<Vec<_>>().join(";"))
    }

    let (light, dark) = (self.theme_vars(Mode::Light), self.theme_vars(Mode::Dark));

    format!(
      ":root{{--accent-base:{}}}{}{}@media (prefers-color-scheme:dark){{{}}}{}",
      self.accent.to_lowercase(),
      block(":root", &light),
      // A reader who chose light on a dark system must not get the dark overrides.
      block(":root[data-theme=\"dark\"]", &dark),
      block(":root:not([data-theme=\"light\"])", &dark),
      // Last, so it wins over everything above at equal specificity.
      self.custom_css,
    )
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
      Settings {
        light: ModeColors {
          text: Some("red".into()),
          ..Default::default()
        },
        ..ok.clone()
      },
      Settings {
        header_links: vec![SiteLink {
          label: "x".into(),
          url: "javascript:alert(1)".into(),
          icon: None,
        }],
        ..ok.clone()
      },
      Settings {
        header_links: vec![SiteLink {
          label: " ".into(),
          url: "https://github.com".into(),
          icon: Some("github".into()),
        }],
        ..ok.clone()
      },
      Settings {
        header_links: vec![SiteLink {
          label: "GitHub".into(),
          url: "https://github.com".into(),
          icon: Some("<svg>".into()),
        }],
        ..ok.clone()
      },
      Settings {
        banner: Banner {
          text: "New".into(),
          url: "javascript:alert(1)".into(),
        },
        ..ok.clone()
      },
    ] {
      assert!(bad.validate().is_err(), "{bad:?}");
    }
  }

  #[test]
  fn refuses_a_low_contrast_palette_and_names_the_pair() {
    let faint = Settings {
      light: ModeColors {
        text: Some("#cccccc".into()),
        ..Default::default()
      },
      ..Settings::default()
    };
    let problems = faint.contrast_problems();
    assert!(problems.iter().any(|p| p.starts_with("Text on the background in light mode")), "{problems:?}");
    assert!(faint.validate().unwrap_err().contains("contrast"));

    // The defaults and a pale accent in dark mode (derived lighter, so still readable) pass.
    assert!(Settings::default().contrast_problems().is_empty());
    assert!(
      Settings {
        accent: "#ffff00".into(),
        ..Settings::default()
      }
      .contrast_problems()
      .iter()
      .all(|p| p.contains("light mode"))
    );
  }

  #[test]
  fn compiles_overrides_per_mode() {
    let settings = Settings {
      accent: "#AA3300".into(),
      dark: ModeColors {
        background: Some("#000000".into()),
        ..Default::default()
      },
      ..Settings::default()
    };
    let css = settings.theme_css();

    assert!(css.starts_with(":root{--accent-base:#aa3300}"), "{css}");
    assert!(css.contains(":root[data-theme=\"dark\"]{--bg:#000000}"), "{css}");
    assert!(css.contains("@media (prefers-color-scheme:dark){:root:not([data-theme=\"light\"]){--bg:#000000}}"), "{css}");
    assert!(!css.contains("--text"), "{css}");

    let custom = Settings {
      custom_css: ".site-header{height:60px}".into(),
      ..settings
    };
    assert!(custom.theme_css().ends_with("}.site-header{height:60px}"), "{}", custom.theme_css());
  }
}
