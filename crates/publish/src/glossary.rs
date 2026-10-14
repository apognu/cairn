use std::collections::HashMap;

use cairn_core::{Document, Library, PageId};
use serde_json::{Value, json};

/// The generated Glossary page: each term as a heading (anchored as `term_anchors` says), its definition, its other words, and a link to
/// the page that explains it in full when that page is published. `terms` is the order the page lists them in.
pub(crate) fn glossary_doc(terms: &[String], library: &Library, urls: &HashMap<PageId, String>) -> Document {
  let text = |text: &str| json!({ "type": "text", "text": text });
  let mut blocks: Vec<Value> = Vec::new();

  for name in terms {
    let Some(term) = library.terms.get(name) else { continue };

    blocks.push(json!({ "type": "heading", "attrs": { "level": 2 }, "content": [text(name)] }));
    blocks.extend(term.definition.0.content.iter().flatten().filter_map(|block| serde_json::to_value(block).ok()));
    if !term.aliases.is_empty() {
      blocks.push(json!({ "type": "paragraph", "content": [text("Also called "), { "type": "text", "text": term.aliases.join(", "), "marks": [{ "type": "italic" }] }, text(".")] }));
    }
    if let Some(page) = term.page
      && let Some(url) = urls.get(&page)
    {
      let link = json!({ "type": "link", "attrs": { "href": url, "pageId": page } });
      blocks.push(json!({ "type": "paragraph", "content": [{ "type": "text", "text": format!("Read more about {name}"), "marks": [link] }] }));
    }
  }

  serde_json::from_value(json!({ "type": "doc", "content": blocks })).unwrap_or_else(|_| Document::empty())
}
