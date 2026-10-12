use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A page body: the ProseMirror JSON of the editor, stored and rendered as is.
///
/// Block nodes carry a stable `attrs.id` (Tiptap UniqueID); internal links and components store page IDs and props in `attrs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(value_type = serde_json::Value))]
pub struct Document(pub Node);

impl Document {
  /// A document with a single empty paragraph: what a new page starts as.
  pub fn empty() -> Self {
    Document(Node {
      kind: "doc".into(),
      attrs: None,
      content: Some(vec![Node {
        kind: "paragraph".into(),
        attrs: None,
        content: None,
        marks: None,
        text: None,
      }]),
      marks: None,
      text: None,
    })
  }
}

/// A ProseMirror node. Absent fields stay absent when serialized, so a document comes back unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
  #[serde(rename = "type")]
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub attrs: Option<Map<String, Value>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub content: Option<Vec<Node>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub marks: Option<Vec<Mark>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
}

/// An inline mark (bold, link, code…) on a text node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mark {
  #[serde(rename = "type")]
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub attrs: Option<Map<String, Value>>,
}

impl Node {
  /// The block ID, if the node has one.
  pub fn id(&self) -> Option<&str> {
    self.attrs.as_ref()?.get("id")?.as_str()
  }

  /// This node and all its descendants, depth first.
  pub fn walk(&self) -> Vec<&Node> {
    let mut nodes = vec![self];

    for child in self.content.iter().flatten() {
      nodes.extend(child.walk());
    }

    nodes
  }
}

#[cfg(test)]
mod tests {
  use std::fs;

  use super::*;

  // Other parts depend on this format: a stored document must come back byte-for-byte equivalent.
  #[test]
  fn every_fixture_round_trips_unchanged() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
    let mut count = 0;

    for entry in fs::read_dir(dir).unwrap() {
      let path = entry.unwrap().path();

      if path.extension().is_none_or(|ext| ext != "json") {
        continue;
      }

      let original: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
      let document: Document = serde_json::from_value(original.clone()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));

      assert_eq!(serde_json::to_value(&document).unwrap(), original, "{}", path.display());
      count += 1;
    }

    assert!(count >= 2, "fixtures not found");
  }

  #[test]
  fn finds_block_ids() {
    let doc: Document = serde_json::from_str(include_str!("../../../fixtures/article.json")).unwrap();
    let ids: Vec<_> = doc.0.walk().into_iter().filter_map(Node::id).collect();

    assert_eq!(ids.first(), Some(&"b1"));
    assert!(ids.len() > 5);
  }
}
