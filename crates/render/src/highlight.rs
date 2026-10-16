use std::sync::LazyLock;

use syntect::{
  highlighting::{FontStyle, Highlighter, Style},
  html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style},
  parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet},
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

const LIGHT: EmbeddedThemeName = EmbeddedThemeName::OneHalfLight;
const DARK: EmbeddedThemeName = EmbeddedThemeName::OneHalfDark;

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

/// A run of code drawn in one style, for the editor's canvas. Offsets count UTF-16 code units, as JavaScript strings do.
#[derive(Debug, PartialEq)]
pub struct Token {
  pub from: usize,
  pub to: usize,
  /// None where the theme draws the text in its default style, which the site's own text color stands in for.
  pub light: Option<TokenStyle>,
  pub dark: Option<TokenStyle>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TokenStyle {
  /// `#rrggbb`.
  pub color: String,
  pub italic: bool,
  pub bold: bool,
}

/// The styled runs of `code`, in the themes the published stylesheet uses. None for a language the highlighter does not know.
///
/// The published page nests one span per scope and lets the stylesheet's descendant selectors pick a color; the editor's decorations
/// cannot nest like that, so each run's style is resolved here instead.
pub fn highlight_tokens(language: &str, code: &str) -> Option<Vec<Token>> {
  let syntax = syntax(language)?;
  let themes = [Highlighter::new(THEMES.get(LIGHT)), Highlighter::new(THEMES.get(DARK))];
  let plain = themes.each_ref().map(Highlighter::get_default);

  let mut parser = ParseState::new(syntax);
  let mut stack = ScopeStack::new();
  let mut tokens: Vec<Token> = Vec::new();
  let mut offset = 0;

  for line in LinesWithEndings::from(code) {
    let ops = parser.parse_line(line, &SYNTAXES).ok()?;
    let mut start = 0;

    // Each op changes the scopes from its byte index on; the text between two ops has the scopes of the first.
    for (end, op) in ops.iter().map(|(index, op)| (*index, Some(op))).chain([(line.len(), None)]) {
      let text = &line[start..end];
      let from = offset;
      offset += text.encode_utf16().count();

      if !text.is_empty() {
        let [light, dark] = [0, 1].map(|i| token_style(themes[i].style_for_stack(stack.as_slice()), plain[i]));

        match tokens.last_mut() {
          Some(last) if last.to == from && last.light == light && last.dark == dark => last.to = offset,
          _ if light.is_none() && dark.is_none() => {}
          _ => tokens.push(Token { from, to: offset, light, dark }),
        }
      }

      if let Some(op) = op {
        stack.apply(op).ok()?;
      }
      start = end;
    }
  }

  Some(tokens)
}

fn token_style(style: Style, plain: Style) -> Option<TokenStyle> {
  if style.foreground == plain.foreground && style.font_style == plain.font_style {
    return None;
  }

  let c = style.foreground;
  Some(TokenStyle {
    color: format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
    italic: style.font_style.contains(FontStyle::ITALIC),
    bold: style.font_style.contains(FontStyle::BOLD),
  })
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
  let light = theme_rules(LIGHT);
  let dark = theme_rules(DARK);

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

  #[test]
  fn tokens_count_utf16_and_skip_plain_text() {
    // The emoji is two UTF-16 units: the string after it starts at 13, not 12.
    let tokens = highlight_tokens("rust", "let a = \"😀\"; // hi\nlet b = 1;").expect("rust is known");
    let at = |from: usize| tokens.iter().find(|t| t.from == from);

    let keyword = at(0).expect("`let` is colored");
    assert_eq!(keyword.to, 3);
    assert!(keyword.light.is_some() && keyword.dark.is_some());
    // `a` is plain text and gets no token.
    assert!(tokens.iter().all(|t| !(t.from <= 4 && t.to > 4)), "{tokens:?}");
    let string = at(8).expect("the string is colored");
    assert_eq!(string.to, 12);
    let comment = at(14).expect("the comment is colored");
    // A line comment's scope takes in the line's end.
    assert_eq!(comment.to, 20);
    assert!(at(20).is_some(), "second line's `let`: {tokens:?}");

    assert_eq!(highlight_tokens("nope", "x"), None);
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
