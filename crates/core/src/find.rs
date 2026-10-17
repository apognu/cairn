use serde_json::Value;

use crate::{Document, Node};

/// How many characters of context an excerpt keeps on each side of the match.
const CONTEXT: usize = 40;

/// The words of a document as one line: its text, and the titles, descriptions and labels components keep in their settings.
pub fn plain_text(doc: &Document) -> String {
  let mut words = Vec::new();
  collect(&doc.0, &mut words);

  words.join(" ")
}

fn collect(node: &Node, words: &mut Vec<String>) {
  if let Some(text) = &node.text {
    words.push(text.clone());
  }
  for value in node.attrs.iter().flat_map(|attrs| attrs.iter()).filter(|(key, _)| *key != "id").map(|(_, value)| value) {
    collect_attr(value, false, words);
  }
  // Text nodes of one block run together; blocks are apart.
  let mut inline = String::new();
  for child in node.content.iter().flatten() {
    if child.kind == "text" {
      inline.push_str(child.text.as_deref().unwrap_or_default());
    } else {
      if !inline.is_empty() {
        words.push(std::mem::take(&mut inline));
      }
      collect(child, words);
    }
  }
  if !inline.is_empty() {
    words.push(inline);
  }
}

/// Strings under keys a reader sees (`title`, `description`, `label`), at any depth: card grids keep their cards in a list.
fn collect_attr(value: &Value, readable: bool, words: &mut Vec<String>) {
  match value {
    Value::String(text) if readable && !text.trim().is_empty() => words.push(text.clone()),
    Value::Array(items) => items.iter().for_each(|item| collect_attr(item, readable, words)),
    Value::Object(map) => {
      for (key, inner) in map {
        collect_attr(inner, matches!(key.as_str(), "title" | "description" | "label"), words);
      }
    }
    _ => {}
  }
}

/// The words around the first place `query` appears in `text`, ignoring case, with an ellipsis where it was cut; `None` when it does not.
pub fn excerpt(text: &str, query: &str) -> Option<String> {
  // One lower-case character for each character, so positions in one are positions in the other.
  let lower = |s: &str| s.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect::<Vec<_>>();
  let (chars, haystack, needle) = (text.chars().collect::<Vec<_>>(), lower(text), lower(query.trim()));
  if needle.is_empty() {
    return None;
  }

  let at = haystack.windows(needle.len()).position(|w| w == needle.as_slice())?;
  let (mut start, mut end) = (at.saturating_sub(CONTEXT), (at + needle.len() + CONTEXT).min(chars.len()));
  // Cut between words, not inside one, when there is a space to cut at.
  if start > 0
    && let Some(space) = chars[start..at].iter().position(|c| c.is_whitespace())
  {
    start += space + 1;
  }
  if end < chars.len()
    && let Some(space) = chars[at + needle.len()..end].iter().rposition(|c| c.is_whitespace())
  {
    end = at + needle.len() + space;
  }
  let middle: String = chars[start..end].iter().collect();

  Some(format!("{}{}{}", if start > 0 { "…" } else { "" }, middle.trim(), if end < chars.len() { "…" } else { "" }))
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn reads_text_and_component_titles() {
    let doc: Document = serde_json::from_value(json!({ "type": "doc", "content": [
      { "type": "heading", "attrs": { "id": "h1", "level": 2 }, "content": [{ "type": "text", "text": "Install" }] },
      { "type": "paragraph", "content": [{ "type": "text", "text": "Run " }, { "type": "text", "text": "cairn", "marks": [{ "type": "code" }] }, { "type": "text", "text": " now." }] },
      { "type": "cards", "attrs": { "columns": "2", "cards": [{ "icon": "star", "title": "Fast", "description": "Very fast", "page": "x" }] } },
    ] }))
    .unwrap();

    assert_eq!(plain_text(&doc), "Install Run cairn now. Very fast Fast");
  }

  #[test]
  fn cuts_an_excerpt_around_the_match() {
    let long = format!("{} the needle is here {}", "a".repeat(60), "b".repeat(60));
    let words = "one two three four five six seven eight nine ten needle eleven twelve thirteen fourteen fifteen sixteen";
    let cases = [
      ("Short text with Needle inside", "needle", Some("Short text with Needle inside".to_string())),
      // The words cut in two at either end are left out.
      (long.as_str(), "NEEDLE", Some("…the needle is here…".to_string())),
      (words, "needle", Some("…four five six seven eight nine ten needle eleven twelve thirteen fourteen…".to_string())),
      ("Café crème", "crème", Some("Café crème".to_string())),
      ("nothing", "needle", None),
      ("anything", "  ", None),
    ];

    for (text, query, expected) in cases {
      assert_eq!(excerpt(text, query), expected, "{query}");
    }
  }
}
