//! The picture link previews show for a page (`og:image`): the site's logo or name, the page title and description, drawn as SVG and
//! rasterized at publish time.

use std::sync::{Arc, LazyLock};

use anyhow::Context;
use base64::Engine;
use resvg::{tiny_skia, usvg};
use sha2::{Digest, Sha256};
use skrifa::{FontRef, MetadataProvider, instance::Size};

use crate::generated::xml_escape;

/// The size every social network crops to without cutting the text.
pub(crate) const WIDTH: u32 = 1200;
pub(crate) const HEIGHT: u32 = 630;

/// Part of every card's cache key, bumped when the drawing changes.
const CARD_FORMAT: &str = "1";

// The site's own font, so cards look like the pages. No system fonts are loaded: a card must come out the same on every server.
const FONT: &[u8] = include_bytes!("../fonts/HankenGrotesk.ttf");
const FAMILY: &str = "Hanken Grotesk";

static FONTS: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
  let mut fonts = usvg::fontdb::Database::new();
  fonts.load_font_data(FONT.to_vec());
  Arc::new(fonts)
});

const PADDING: f32 = 80.0;
const TEXT_WIDTH: f32 = WIDTH as f32 - 2.0 * PADDING;
// `--muted` in light mode; the theme does not change it.
const MUTED: &str = "#5b6168";

/// What every card of a site shares.
pub(crate) struct CardStyle {
  pub site_name: String,
  /// The site's address without its scheme, at the bottom of the card.
  pub host: String,
  pub background: String,
  pub text: String,
  pub accent: String,
  pub logo: Option<Logo>,
}

/// The light-mode logo, embedded in the card.
pub(crate) struct Logo {
  pub bytes: Vec<u8>,
  pub mime: String,
  pub width: u32,
  pub height: u32,
}

/// The cache key of a page's card: a hash of everything drawn on it.
pub(crate) fn key(style: &CardStyle, title: &str, description: Option<&str>) -> String {
  let mut digest = Sha256::new();
  for part in [
    CARD_FORMAT,
    title,
    description.unwrap_or_default(),
    &style.site_name,
    &style.host,
    &style.background,
    &style.text,
    &style.accent,
  ] {
    digest.update(part.as_bytes());
    digest.update([0]);
  }
  if let Some(logo) = &style.logo {
    digest.update(&logo.bytes);
  }

  digest.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The card as a PNG. CPU bound: run it off the async threads.
pub(crate) fn render(style: &CardStyle, title: &str, description: Option<&str>) -> anyhow::Result<Vec<u8>> {
  let svg = svg(style, title, description);

  let options = usvg::Options {
    fontdb: FONTS.clone(),
    font_family: FAMILY.into(),
    ..usvg::Options::default()
  };
  let tree = usvg::Tree::from_str(&svg, &options).context("drawing the link preview card")?;
  let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).context("allocating the link preview card")?;
  resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

  pixmap.encode_png().context("encoding the link preview card")
}

fn svg(style: &CardStyle, title: &str, description: Option<&str>) -> String {
  let mut body = String::new();

  // The logo usually carries the name already; without one, the name stands in its place.
  match &style.logo {
    Some(logo) => {
      let height = 64.0_f32;
      let width = (logo.width as f32 * height / logo.height.max(1) as f32).min(TEXT_WIDTH / 2.0);
      let data = base64::engine::general_purpose::STANDARD.encode(&logo.bytes);
      body += &format!(
        r#"<image x="{PADDING}" y="{PADDING}" width="{width}" height="{height}" preserveAspectRatio="xMinYMid meet" href="data:{};base64,{data}"/>"#,
        logo.mime
      );
    }
    None => body += &text_line(PADDING + 44.0, 40.0, 600, &style.text, &style.site_name),
  }

  // Two lines of large type when the title fits in them, three of smaller type, starting higher, otherwise.
  let (size, mut y, lines) = match wrap(title, 72.0, 700, 2) {
    lines if !lines.last().is_some_and(|l| l.ends_with('…')) => (72.0, 260.0, lines),
    _ => (60.0, 230.0, wrap(title, 60.0, 700, 3)),
  };
  for line in &lines {
    body += &text_line(y, size, 700, &style.text, line);
    y += size * 1.15;
  }

  // The description takes the lines left above the address, two at most.
  let footer = HEIGHT as f32 - PADDING + 8.0;
  if let Some(description) = description.filter(|d| !d.trim().is_empty()) {
    y += 12.0;
    let room = ((footer - 56.0 - y) / (32.0 * 1.35)).floor().max(-1.0) as isize + 1;
    for line in wrap(description, 32.0, 400, room.clamp(0, 2) as usize) {
      body += &text_line(y, 32.0, 400, MUTED, &line);
      y += 32.0 * 1.35;
    }
  }

  body += &text_line(footer, 28.0, 500, MUTED, &style.host);

  format!(
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}"><rect width="{WIDTH}" height="{HEIGHT}" fill="{}"/><rect width="{WIDTH}" height="14" fill="{}"/>{body}</svg>"#,
    style.background, style.accent
  )
}

fn text_line(baseline: f32, size: f32, weight: u16, color: &str, text: &str) -> String {
  format!(
    r#"<text x="{PADDING}" y="{baseline}" font-family="{FAMILY}" font-size="{size}" font-weight="{weight}" fill="{color}">{}</text>"#,
    xml_escape(text)
  )
}

/// Breaks `text` into at most `max_lines` lines that fit the card at this size, on spaces where it can. What does not fit ends the last
/// line with an ellipsis.
fn wrap(text: &str, size: f32, weight: u16, max_lines: usize) -> Vec<String> {
  if max_lines == 0 {
    return Vec::new();
  }

  let fits = |line: &str| width(line, size, weight) <= TEXT_WIDTH;
  let mut lines: Vec<String> = Vec::new();
  let mut line = String::new();

  for word in text.split_whitespace() {
    let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
    if fits(&candidate) {
      line = candidate;
      continue;
    }
    if !line.is_empty() {
      lines.push(std::mem::take(&mut line));
    }
    // A word wider than the card on its own is cut where it overflows.
    for c in word.chars() {
      if !line.is_empty() && !fits(&format!("{line}{c}")) {
        lines.push(std::mem::take(&mut line));
      }
      line.push(c);
    }
  }
  if !line.is_empty() {
    lines.push(line);
  }

  if lines.len() > max_lines {
    lines.truncate(max_lines);
    let last = &mut lines[max_lines - 1];
    while !last.is_empty() && !fits(&format!("{last}…")) {
      last.pop();
    }
    *last = format!("{}…", last.trim_end());
  }

  lines
}

/// How wide `text` is drawn in the card's font, from the font's own advances at that weight.
fn width(text: &str, size: f32, weight: u16) -> f32 {
  let font = FontRef::new(FONT).expect("the bundled font is a valid font file");
  let location = font.axes().location([("wght", f32::from(weight))]);
  let metrics = font.glyph_metrics(Size::new(size), &location);
  let charmap = font.charmap();

  // A character the font lacks is drawn as an empty box about half as wide as the type is tall.
  text.chars().map(|c| charmap.map(c).and_then(|glyph| metrics.advance_width(glyph)).unwrap_or(size / 2.0)).sum()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn titles_wrap_on_spaces_and_what_does_not_fit_ends_in_an_ellipsis() {
    let long = "Configure single sign-on with your identity provider, then invite the rest of the team and give them roles";
    // The text, the most lines, how many lines it takes and whether it was cut.
    let cases = [
      ("Install", 2, 1, false),
      ("Getting started with the command line", 2, 2, false),
      (long, 2, 2, true),
      (long, 3, 3, true),
      ("Roles", 3, 1, false),
    ];

    for (text, max_lines, count, cut) in cases {
      let lines = wrap(text, 72.0, 700, max_lines);
      assert!(lines.iter().all(|l| width(l, 72.0, 700) <= TEXT_WIDTH), "{lines:?}");
      assert_eq!((lines.len(), lines.last().is_some_and(|l| l.ends_with('…'))), (count, cut), "{lines:?}");
    }
  }

  #[test]
  fn a_word_wider_than_the_card_is_cut() {
    let lines = wrap(&"a".repeat(200), 72.0, 700, 3);

    assert_eq!(lines.len(), 3);
    assert!(lines[2].ends_with('…'));
  }
}
