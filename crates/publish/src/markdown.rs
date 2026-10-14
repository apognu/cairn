//! The Markdown version of a page, for people and language models that want the text without the site around it.

use std::collections::HashMap;

use cairn_core::{Document, Mark, Node};
use serde_json::{Map, Value};

/// What the converter needs to know about the rest of the site.
pub(crate) struct Site<'a> {
  /// `https://docs.example.com`, without a trailing slash; links in the Markdown are absolute so the file stands alone.
  pub url: &'a str,
  /// Page ID to its path (`/guide/install/`).
  pub urls: &'a HashMap<String, String>,
  /// Page ID to its title.
  pub titles: &'a HashMap<String, String>,
  /// Asset ID to the path of its published file.
  pub assets: &'a HashMap<String, String>,
}

/// The page as Markdown: its title as the heading, then the body.
pub(crate) fn page_markdown(title: &str, description: Option<&str>, doc: &Document, site: &Site) -> String {
  let mut out = format!("# {title}\n\n");
  if let Some(description) = description {
    out.push_str(&format!("{description}\n\n"));
  }
  out.push_str(&blocks(doc.0.content.as_deref().unwrap_or_default(), site, ""));

  format!("{}\n", out.trim_end())
}

/// Blocks separated by blank lines, every line prefixed by `indent` (for list items and quotes the caller indents itself).
fn blocks(nodes: &[Node], site: &Site, indent: &str) -> String {
  let parts: Vec<String> = nodes.iter().map(|n| block(n, site)).filter(|s| !s.trim().is_empty()).collect();

  parts
    .join("\n\n")
    .lines()
    .map(|l| if l.is_empty() { String::new() } else { format!("{indent}{l}") })
    .collect::<Vec<_>>()
    .join("\n")
}

fn attr<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
  node.attrs.as_ref()?.get(name)
}

fn attr_str<'a>(node: &'a Node, name: &str) -> &'a str {
  attr(node, name).and_then(Value::as_str).unwrap_or_default()
}

fn block(node: &Node, site: &Site) -> String {
  let children = node.content.as_deref().unwrap_or_default();

  match node.kind.as_str() {
    // A paragraph ending in a hard break would leave a stray blank line.
    "paragraph" => inline(children, site).trim_end().lines().map(escape_line_start).collect::<Vec<_>>().join("\n"),
    "heading" => {
      let level = attr(node, "level").and_then(Value::as_u64).unwrap_or(2).clamp(1, 6);
      // The page title is the one H1; headings in the body start at H2 so the file has a single top level.
      format!("{} {}", "#".repeat(level.max(2) as usize), inline(children, site))
    }
    "bulletList" => list(children, site, |_| "- ".into()),
    "orderedList" => list(children, site, |i| format!("{}. ", i + 1)),
    "blockquote" => blocks(children, site, "> "),
    "codeBlock" => fence(attr_str(node, "language"), &text_of(node)),
    "horizontalRule" => "---".into(),
    "image" => {
      let id = attr_str(node, "assetId");
      let src = site.assets.get(id).map_or_else(|| format!("/_a/{id}"), |p| format!("/{p}"));
      let caption = attr_str(node, "caption");
      let image = format!("![{}]({}{src})", escape(attr_str(node, "alt")), site.url);

      if caption.is_empty() { image } else { format!("{image}\n*{}*", escape(caption)) }
    }
    "table" => table(children, site),
    "callout" => {
      let tone = attr_str(node, "tone");
      let label = match tone {
        "warning" => "Warning",
        "tip" => "Tip",
        "danger" => "Danger",
        _ => "Note",
      };
      let body = blocks(children, site, "> ");

      format!("> **{label}**\n>\n{body}")
    }
    "tabs" => children
      .iter()
      .map(|panel| format!("**{}**\n\n{}", escape(attr_str(panel, "label")), blocks(panel.content.as_deref().unwrap_or_default(), site, "")))
      .collect::<Vec<_>>()
      .join("\n\n"),
    "steps" => children
      .iter()
      .enumerate()
      .map(|(i, step)| {
        let mut parts = step.content.as_deref().unwrap_or_default().iter();
        let title = parts.next().map(|t| inline(t.content.as_deref().unwrap_or_default(), site)).unwrap_or_default();
        let rest: Vec<Node> = parts.cloned().collect();
        let body = blocks(&rest, site, "   ");

        if body.is_empty() {
          format!("{}. **{title}**", i + 1)
        } else {
          format!("{}. **{title}**\n\n{body}", i + 1)
        }
      })
      .collect::<Vec<_>>()
      .join("\n"),
    "fileTree" => fence("", attr_str(node, "tree")),
    "cards" => cards(attr(node, "cards"), site),
    "featureTabs" => {
      let families = attr(node, "families").and_then(Value::as_array).cloned().unwrap_or_default();
      families
        .iter()
        .filter_map(Value::as_object)
        .map(|f| format!("**{}**\n\n{}", escape(field(f, "label")), cards(f.get("features"), site)))
        .collect::<Vec<_>>()
        .join("\n\n")
    }
    "whatsNew" => {
      let title = attr_str(node, "title");
      format!("**{}**\n\n{}", escape(if title.is_empty() { "What's new" } else { title }), cards(attr(node, "items"), site))
    }
    "linkCard" => page_links(attr(node, "page").into_iter(), site),
    "relatedPages" => {
      let title = attr_str(node, "title");
      let pages = attr(node, "pages").and_then(Value::as_array).cloned().unwrap_or_default();
      format!("**{}**\n\n{}", escape(if title.is_empty() { "Related" } else { title }), page_links(pages.iter(), site))
    }
    "seeAlso" => {
      let pages = attr(node, "pages").and_then(Value::as_array).cloned().unwrap_or_default();
      format!("**See also**\n\n{}", page_links(pages.iter(), site))
    }
    // A component this converter does not know still contributes whatever text it holds.
    _ => blocks(children, site, ""),
  }
}

fn field<'a>(map: &'a Map<String, Value>, name: &str) -> &'a str {
  map.get(name).and_then(Value::as_str).unwrap_or_default()
}

fn list(items: &[Node], site: &Site, marker: impl Fn(usize) -> String) -> String {
  items
    .iter()
    .enumerate()
    .map(|(i, item)| {
      let marker = marker(i);
      let pad = " ".repeat(marker.len());
      let body = blocks(item.content.as_deref().unwrap_or_default(), site, &pad);

      format!("{marker}{}", body.trim_start()).trim_end().to_string()
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn table(rows: &[Node], site: &Site) -> String {
  let cells = |row: &Node| -> Vec<String> {
    row
      .content
      .as_deref()
      .unwrap_or_default()
      .iter()
      .map(|c| blocks(c.content.as_deref().unwrap_or_default(), site, "").replace('\n', " ").replace('|', "\\|"))
      .collect()
  };
  let mut lines: Vec<String> = rows.iter().map(|r| format!("| {} |", cells(r).join(" | "))).collect();

  // GitHub tables need a header separator; the first row is the header even when the editor did not mark it.
  if let Some(first) = rows.first() {
    lines.insert(1, format!("|{}|", vec![" --- "; cells(first).len().max(1)].join("|")));
  }

  lines.join("\n")
}

/// A bulleted list of the cards of a component: `- [Title](url): description`, or plain text when there is no link.
fn cards(items: Option<&Value>, site: &Site) -> String {
  items
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(Value::as_object)
        .map(|card| {
          let title = escape(field(card, "title"));
          let href = card
            .get("page")
            .and_then(Value::as_str)
            .and_then(|id| site.urls.get(id))
            .map(|path| format!("{}{path}", site.url))
            .or_else(|| Some(field(card, "href").to_string()).filter(|h| !h.is_empty()));
          let head = href.map_or_else(|| title.clone(), |href| format!("[{title}]({href})"));
          let description = field(card, "description");

          if description.is_empty() {
            format!("- {head}")
          } else {
            format!("- {head}: {}", escape(description))
          }
        })
        .collect::<Vec<_>>()
        .join("\n")
    })
    .unwrap_or_default()
}

/// Links to the pages with these IDs; pages that no longer exist are left out.
fn page_links<'a>(ids: impl Iterator<Item = &'a Value>, site: &Site) -> String {
  ids
    .filter_map(Value::as_str)
    .filter_map(|id| Some(format!("- [{}]({}{})", escape(site.titles.get(id)?), site.url, site.urls.get(id)?)))
    .collect::<Vec<_>>()
    .join("\n")
}

fn fence(language: &str, code: &str) -> String {
  // A fence longer than any run of backticks inside cannot be closed by the code itself.
  let longest = code.split(|c| c != '`').map(str::len).max().unwrap_or(0);
  let ticks = "`".repeat(longest.max(2) + 1);

  format!("{ticks}{language}\n{code}\n{ticks}")
}

fn text_of(node: &Node) -> String {
  node.text.clone().unwrap_or_else(|| node.content.iter().flatten().map(text_of).collect())
}

/// Characters that would start Markdown syntax in running text.
fn escape(text: &str) -> String {
  let mut out = String::with_capacity(text.len());
  for c in text.chars() {
    if matches!(c, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>') {
      out.push('\\');
    }
    out.push(c);
  }
  out
}

/// A line of running text that would read as a heading, a list item or a rule: `# 1`, `- a`, `+ b`, `2. c`.
fn escape_line_start(line: &str) -> String {
  // Every character counted below is ASCII, so counts are byte offsets.
  let after = |n: usize| line[n..].chars().next().is_none_or(char::is_whitespace);
  let first = line.chars().next();
  let run = line.chars().take_while(|c| Some(*c) == first).count();
  let digits = line.chars().take_while(char::is_ascii_digit).count();

  if matches!(first, Some('#' | '-' | '+' | '=')) && after(run) {
    format!("\\{line}")
  } else if (1..=9).contains(&digits) && line[digits..].starts_with(['.', ')']) && after(digits + 1) {
    format!("{}\\{}", &line[..digits], &line[digits..])
  } else {
    line.to_string()
  }
}

/// Inline code: a delimiter longer than any run of backticks inside, padded when the code itself starts or ends with one.
fn code_span(text: &str) -> String {
  let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
  let ticks = "`".repeat(longest + 1);
  let pad = if text.starts_with('`') || text.ends_with('`') { " " } else { "" };

  format!("{ticks}{pad}{text}{pad}{ticks}")
}

fn inline(nodes: &[Node], site: &Site) -> String {
  nodes.iter().map(|n| inline_node(n, site)).collect()
}

fn inline_node(node: &Node, site: &Site) -> String {
  match node.kind.as_str() {
    "text" => {
      let text = node.text.as_deref().unwrap_or_default();
      let marks = node.marks.as_deref().unwrap_or_default();

      // Inside code nothing is escaped; elsewhere the text is, and the marks wrap it from the outside in.
      let mut out = if marks.iter().any(|m| m.kind == "code") { code_span(text) } else { escape(text) };
      for mark in marks.iter().rev() {
        out = wrap(out, mark, site);
      }
      out
    }
    "hardBreak" => "  \n".into(),
    _ => inline(node.content.as_deref().unwrap_or_default(), site),
  }
}

fn wrap(inner: String, mark: &Mark, site: &Site) -> String {
  let attr = |name: &str| mark.attrs.as_ref().and_then(|a| a.get(name)).and_then(Value::as_str);

  match mark.kind.as_str() {
    "bold" => format!("**{inner}**"),
    "italic" => format!("*{inner}*"),
    "strike" => format!("~~{inner}~~"),
    "link" => {
      // A link to a section of a page keeps its anchor, as on the site.
      let anchor = attr("anchor").map(|a| format!("#{a}")).unwrap_or_default();
      let by_id = attr("pageId").and_then(|id| site.urls.get(id)).map(|path| format!("{}{path}{anchor}", site.url));
      // A link to a page that is gone has no destination: only its words remain.
      match (attr("pageId"), by_id, attr("href")) {
        (Some(_), None, _) => inner,
        (_, Some(url), _) => format!("[{inner}]({url})"),
        (None, None, Some(href)) => format!("[{inner}]({href}{anchor})"),
        _ => inner,
      }
    }
    _ => inner,
  }
}

#[cfg(test)]
mod tests {
  use std::path::Path;

  use super::*;

  fn convert(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(format!("{name}.json"));
    let doc: Document = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let (urls, titles, assets) = (HashMap::new(), HashMap::new(), HashMap::new());

    page_markdown(
      "Fixture",
      None,
      &doc,
      &Site {
        url: "https://docs.example.com",
        urls: &urls,
        titles: &titles,
        assets: &assets,
      },
    )
  }

  // Each fixture exercises one kind of block; its Markdown should read the way a person would have written it.
  #[test]
  fn every_fixture_reads_correctly() {
    let cases: [(&str, &[&str]); 7] = [
      (
        "tabs",
        &["# Fixture\n\n## Install\n\nPick your platform.", "**Linux**\n\nUse the package manager.", "**macOS**\n\nUse Homebrew."],
      ),
      (
        "steps",
        &[
          "1. **Open the project settings**\n\n   In the dashboard, choose your project, then Settings.",
          "2. **Add the endpoint URL**",
          "3. **Store the signing secret**\n\nThat is all.",
        ],
      ),
      (
        "cards",
        &[
          "- [Quickstart](https://example.com/quickstart): Deploy your first service in five minutes",
          "- Concepts: How releases, environments and rollbacks fit together",
        ],
      ),
      ("filetree", &["```\nsrc/\n  main.rs\n  routes/\n    mod.rs\n    health.rs\nCargo.toml\nREADME.md\n```"]),
      (
        "codegroup",
        &["**npm**\n\n```bash\nnpm install --global halyard\n```", "**yarn**\n\n```bash\nyarn global add halyard\n```"],
      ),
      (
        "features",
        &["**Deploying**\n\n- Rolling deploys: Replace instances a few at a time", "- Rollbacks: Previous release back in seconds"],
      ),
      ("article", &["**", "```", "> **"]),
    ];

    for (name, expected) in cases {
      let markdown = convert(name);
      for needle in expected {
        assert!(markdown.contains(needle), "{name}: missing {needle:?} in\n{markdown}");
      }
      assert!(markdown.ends_with('\n') && !markdown.contains("\n\n\n"), "{name}: stray blank lines in\n{markdown}");
    }
  }

  #[test]
  fn text_and_links_are_escaped_and_resolved() {
    let id = "01234567-89ab-7cde-8f01-23456789abcd";
    let doc: Document = serde_json::from_value(serde_json::json!({ "type": "doc", "content": [
      { "type": "paragraph", "content": [
        { "type": "text", "text": "a *star* and " },
        { "type": "text", "text": "a page", "marks": [{ "type": "link", "attrs": { "pageId": id, "href": "/stale/" } }] },
        { "type": "text", "text": " and " },
        { "type": "text", "text": "a gone page", "marks": [{ "type": "link", "attrs": { "pageId": "00000000-0000-7000-8000-000000000000" } }] },
        { "type": "text", "text": "x", "marks": [{ "type": "bold" }, { "type": "italic" }] },
      ] },
      { "type": "paragraph", "content": [
        { "type": "text", "text": "a ", "marks": [{ "type": "link", "attrs": { "pageId": id, "anchor": "setup" } }] },
        { "type": "text", "text": "x`y", "marks": [{ "type": "code" }] },
        { "type": "text", "text": " " },
        { "type": "text", "text": "`", "marks": [{ "type": "code" }] },
      ] },
      { "type": "paragraph", "content": [{ "type": "text", "text": "# not a heading" }, { "type": "hardBreak" }, { "type": "text", "text": "2. not a list" }] },
      { "type": "paragraph", "content": [{ "type": "text", "text": "- not an item, #hashtag and 2.5 stay" }] },
    ] }))
    .unwrap();
    let urls = HashMap::from([(id.to_string(), "/guide/".to_string())]);
    let (titles, assets) = (HashMap::new(), HashMap::new());

    let markdown = page_markdown(
      "T",
      Some("About."),
      &doc,
      &Site {
        url: "https://d.io",
        urls: &urls,
        titles: &titles,
        assets: &assets,
      },
    );

    assert_eq!(
      markdown,
      [
        "# T\n\nAbout.\n",
        "a \\*star\\* and [a page](https://d.io/guide/) and a gone page***x***\n",
        "[a ](https://d.io/guide/#setup)``x`y`` `` ` ``\n",
        "\\# not a heading  \n2\\. not a list\n",
        "\\- not an item, #hashtag and 2.5 stay\n",
      ]
      .join("\n")
    );
  }
}
