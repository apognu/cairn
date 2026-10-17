use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::{Document, PageId};

/// How a page refers to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
  /// A link in the text.
  Inline,
  /// A link card, see-also callout or related-pages component.
  Card,
  /// A page field of any other component.
  Prop,
}

impl LinkKind {
  pub fn as_str(self) -> &'static str {
    match self {
      LinkKind::Inline => "inline",
      LinkKind::Card => "card",
      LinkKind::Prop => "prop",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    [Self::Inline, Self::Card, Self::Prop].into_iter().find(|k| k.as_str() == value)
  }
}

/// One reference from a page to another, as found in the document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageLink {
  pub target: PageId,
  /// A heading anchor on the target page, or empty.
  pub anchor: String,
  pub kind: LinkKind,
}

/// Components that exist to point at other pages; their page fields count as cards, not as plain props.
const CARD_NODES: [&str; 3] = ["linkCard", "seeAlso", "relatedPages"];

/// Every page the document links to. Text links name the page in a `pageId` attribute of the link mark. A component refers to pages
/// in fields called `page` (one page) or `pages` (a list), anywhere in its props.
pub fn extract_links(doc: &Document) -> Vec<PageLink> {
  let mut found = BTreeSet::new();

  for node in doc.0.walk() {
    for mark in node.marks.iter().flatten().filter(|m| m.kind == "link") {
      let attrs = mark.attrs.as_ref();
      let target = attrs.and_then(|a| a.get("pageId")?.as_str()?.parse().ok());
      let anchor = attrs.and_then(|a| a.get("anchor")?.as_str()).unwrap_or_default();

      if let Some(target) = target {
        found.insert(PageLink {
          target,
          anchor: anchor.to_string(),
          kind: LinkKind::Inline,
        });
      }
    }

    if let Some(attrs) = &node.attrs {
      let kind = if CARD_NODES.contains(&node.kind.as_str()) { LinkKind::Card } else { LinkKind::Prop };
      collect_in_map(attrs, kind, &mut found);
    }
  }

  found.into_iter().collect()
}

fn collect_in_map(map: &Map<String, Value>, kind: LinkKind, found: &mut BTreeSet<PageLink>) {
  // A page field holds a page ID, or `<ID>#<anchor>` for an operation of an API page.
  let mut add = |value: &Value| {
    let Some(value) = value.as_str() else { return };
    let (id, anchor) = value.split_once('#').unwrap_or((value, ""));
    if let Ok(target) = id.parse() {
      found.insert(PageLink {
        target,
        anchor: anchor.to_string(),
        kind,
      });
    }
  };

  for (key, child) in map {
    match (key.as_str(), child) {
      ("page", id) => add(id),
      ("pages", Value::Array(ids)) => ids.iter().for_each(&mut add),
      _ => {}
    }
  }

  for child in map.values() {
    match child {
      Value::Object(inner) => collect_in_map(inner, kind, found),
      Value::Array(items) => items.iter().filter_map(Value::as_object).for_each(|inner| collect_in_map(inner, kind, found)),
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn finds_text_links_and_component_fields() {
    let (a, b, c) = (PageId::new(), PageId::new(), PageId::new());
    let doc: Document = serde_json::from_value(json!({ "type": "doc", "content": [
      { "type": "paragraph", "content": [
        { "type": "text", "text": "see", "marks": [{ "type": "link", "attrs": { "pageId": a.to_string(), "href": "/a" } }] },
        { "type": "text", "text": "web", "marks": [{ "type": "link", "attrs": { "href": "https://example.com" } }] },
      ] },
      { "type": "featureTabs", "attrs": { "families": [{ "label": "x", "features": [{ "title": "t", "page": b.to_string() }, { "title": "none" }] }] } },
      { "type": "relatedPages", "attrs": { "pages": [c.to_string(), "not-an-id", format!("{c}#tag/orders/POST/orders")] } },
    ] }))
    .unwrap();

    let links = extract_links(&doc);
    let kinds: Vec<_> = links.iter().map(|l| (l.target, l.kind, l.anchor.as_str())).collect();

    assert_eq!(links.len(), 4);
    assert!(kinds.contains(&(a, LinkKind::Inline, "")) && kinds.contains(&(b, LinkKind::Prop, "")) && kinds.contains(&(c, LinkKind::Card, "")));
    assert!(kinds.contains(&(c, LinkKind::Card, "tag/orders/POST/orders")));
  }
}
