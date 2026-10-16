use std::sync::LazyLock;

use syntect::{
  html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style},
  parsing::{SyntaxReference, SyntaxSet},
  util::LinesWithEndings,
};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

// bat's collection: about 200 languages, where syntect's own defaults lack TypeScript, TOML, Dockerfile, Kotlin and Swift.
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);
static THEMES: LazyLock<EmbeddedLazyThemeSet> = LazyLock::new(two_face::theme::extra);

/// Names written in documents (by the editor, or by other tools in imported pages) that are neither a syntax's name nor one of its file
/// extensions, with a token that finds it.
const ALIASES: &[(&str, &str)] = &[
  ("apache", ".htaccess"),
  ("csharp", "cs"),
  ("dotenv", ".env"),
  ("fsharp", "fs"),
  ("fortran", "f90"),
  ("gdscript", "gd"),
  ("gitconfig", ".gitconfig"),
  ("gitignore", ".gitignore"),
  ("golang", "go"),
  ("gomod", "go.mod"),
  ("graphviz", "dot"),
  ("jinja", "j2"),
  // The TypeScript grammar for JSX reads plain JavaScript JSX too; the JavaScript one does not know JSX.
  ("jsx", "tsx"),
  ("objc", "m"),
  ("objective-c", "m"),
  ("plaintext", "txt"),
  ("regex", "re"),
  ("requirements", "requirements.txt"),
  ("shell", "sh"),
  ("text", "txt"),
];

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
  let syntax = syntax(language)?;
  let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAXES, STYLE);

  for line in LinesWithEndings::from(code) {
    generator.parse_html_for_line_which_includes_newline(line).ok()?;
  }

  Some(generator.finalize())
}

fn syntax(language: &str) -> Option<&'static SyntaxReference> {
  let language = language.to_lowercase();
  let token = ALIASES.iter().find(|(alias, _)| *alias == language).map_or(language.as_str(), |(_, token)| token);

  SYNTAXES.find_syntax_by_token(token)
}

/// Reverses the escaping React applies to text children.
fn unescape(text: &str) -> String {
  text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#x27;", "'").replace("&amp;", "&")
}

/// The stylesheet for the classes above: a light theme by default, a dark one under the system setting or an explicit `data-theme="dark"`.
pub fn highlight_css() -> String {
  let light = theme_rules(EmbeddedThemeName::OneHalfLight);
  let dark = theme_rules(EmbeddedThemeName::OneHalfDark);

  // Each theme applies only in its own mode. Scoping one and not the other lets the unscoped theme win wherever its selector has more
  // classes, so light colors leaked into dark mode.
  format!(
    "@media not (prefers-color-scheme: dark) {{\n{}}}\n{}@media (prefers-color-scheme: dark) {{\n{}}}\n{}",
    scoped(&light, ":root:not([data-theme=\"dark\"])"),
    scoped(&light, ":root[data-theme=\"light\"]"),
    scoped(&dark, ":root:not([data-theme=\"light\"])"),
    scoped(&dark, ":root[data-theme=\"dark\"]"),
  )
}

fn theme_rules(name: EmbeddedThemeName) -> String {
  let theme = THEMES.get(name);

  css_for_theme_with_class_style(theme, STYLE)
    .unwrap_or_default()
    .lines()
    .filter(|line| !line.contains("background-color"))
    .collect::<Vec<_>>()
    .join("\n")
}

/// Puts `prefix` before every selector of every rule; a rule's selector list can span several lines.
fn scoped(rules: &str, prefix: &str) -> String {
  let mut out = String::new();

  for rule in rules.split_inclusive('}') {
    let Some((selectors, body)) = rule.split_once('{') else { continue };
    // Comments (the theme's name) sit before the first selector.
    let selectors = selectors.rsplit_once("*/").map_or(selectors, |(_, after)| after);
    let selectors = selectors.split(',').map(|s| format!("{prefix} {}", s.trim())).collect::<Vec<_>>().join(", ");

    out.push_str(&format!("{selectors} {{{body}\n"));
  }

  out
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

  /// Every language the editor offers is one the highlighter knows.
  #[test]
  fn knows_the_editor_languages() {
    let languages: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../packages/editor/src/languages.json")).expect("languages.json is JSON");
    let unknown: Vec<_> = languages.iter().filter_map(|l| l["value"].as_str()).filter(|value| syntax(value).is_none()).collect();

    assert!(unknown.is_empty(), "no syntax for {unknown:?}");
  }

  #[test]
  fn css_has_light_and_dark_rules() {
    let css = highlight_css();

    assert!(css.contains("@media (prefers-color-scheme: dark)"));
    assert!(css.contains(":root[data-theme=\"dark\"] .hl-"), "{css}");
    assert!(!css.contains("background-color"));

    // Every selector is scoped to one mode, or the theme with the longer selectors wins in both.
    for rule in css.split('}') {
      let Some((head, _)) = rule.rsplit_once('{') else { continue };
      let selectors = head.rsplit_once('{').map_or(head, |(_, inner)| inner);
      for selector in selectors.split(',') {
        assert!(selector.trim().starts_with(":root"), "unscoped selector {selector:?}");
      }
    }
  }
}
