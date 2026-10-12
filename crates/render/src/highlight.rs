use std::sync::LazyLock;

use syntect::{
  highlighting::ThemeSet,
  html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style},
  parsing::SyntaxSet,
  util::LinesWithEndings,
};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

// Classes instead of inline colors, so one stylesheet serves light and dark mode.
const STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };

const OPEN: &str = "<code class=\"language-";

/// Colors the contents of every `<code class="language-x">` block in rendered HTML. Blocks in an unknown language are left as they are.
pub fn highlight_code_blocks(html: &str) -> String {
  let mut out = String::with_capacity(html.len());
  let mut rest = html;

  while let Some(start) = rest.find(OPEN) {
    let after_open = &rest[start + OPEN.len()..];

    // The renderer's output is regular: `language-x">text</code>`. Anything else is left alone.
    let (Some(quote), Some(end)) = (after_open.find("\">"), after_open.find("</code>")) else { break };
    if quote > end {
      break;
    }

    let language = &after_open[..quote];
    let text = &after_open[quote + 2..end];
    out.push_str(&rest[..start + OPEN.len() + quote + 2]);

    match highlight(language, &unescape(text)) {
      Some(colored) => out.push_str(&colored),
      None => out.push_str(text),
    }

    rest = &after_open[end..];
  }

  out.push_str(rest);
  out
}

fn highlight(language: &str, code: &str) -> Option<String> {
  let syntax = SYNTAXES.find_syntax_by_token(language)?;
  let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAXES, STYLE);

  for line in LinesWithEndings::from(code) {
    generator.parse_html_for_line_which_includes_newline(line).ok()?;
  }

  Some(generator.finalize())
}

/// Reverses the escaping React applies to text children.
fn unescape(text: &str) -> String {
  text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#x27;", "'").replace("&amp;", "&")
}

/// The stylesheet for the classes above: a light theme by default, a dark one under the system setting or an explicit `data-theme="dark"`.
pub fn highlight_css() -> String {
  let light = theme_rules("InspiredGitHub");
  let dark = theme_rules("base16-ocean.dark");

  format!(
    "{light}\n@media (prefers-color-scheme: dark) {{\n{}}}\n{}",
    scoped(&dark, ":root:not([data-theme=\"light\"]) "),
    scoped(&dark, ":root[data-theme=\"dark\"] ")
  )
}

fn theme_rules(name: &str) -> String {
  let theme = &THEMES.themes[name];

  // Backgrounds come from the site's own code block style, so only token colors are kept.
  css_for_theme_with_class_style(theme, STYLE)
    .unwrap_or_default()
    .lines()
    .filter(|line| !line.contains("background-color"))
    .collect::<Vec<_>>()
    .join("\n")
}

/// Prefixes every selector line, which syntect emits as `.hl-a.hl-b {`.
fn scoped(rules: &str, prefix: &str) -> String {
  rules
    .lines()
    .map(|line| if line.starts_with('.') { format!("{prefix}{line}\n") } else { format!("{line}\n") })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn colors_known_languages_and_leaves_others() {
    let html = "<pre><code class=\"language-json\">{ &quot;ok&quot;: true }</code></pre><pre><code class=\"language-nope\">a &lt; b</code></pre>";
    let out = highlight_code_blocks(html);

    assert!(out.contains("<span class=\"hl-"), "{out}");
    assert!(out.contains("language-json\"><span"), "{out}");
    assert!(out.ends_with("<code class=\"language-nope\">a &lt; b</code></pre>"), "{out}");
  }

  #[test]
  fn css_has_light_and_dark_rules() {
    let css = highlight_css();

    assert!(css.contains("@media (prefers-color-scheme: dark)"));
    assert!(css.contains(":root[data-theme=\"dark\"] .hl-"), "{css}");
    assert!(!css.contains("background-color"));
  }
}
