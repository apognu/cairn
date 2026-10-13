use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::{Document, Mark, Node};

/// What kind of reusable content an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
  /// A value that is written inline: the latest version, a product name.
  Variable,
  /// A block of content included in many pages.
  Snippet,
  /// A glossary term with a definition.
  Term,
}

impl EntryKind {
  pub fn as_str(self) -> &'static str {
    match self {
      EntryKind::Variable => "variable",
      EntryKind::Snippet => "snippet",
      EntryKind::Term => "term",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    [Self::Variable, Self::Snippet, Self::Term].into_iter().find(|k| k.as_str() == value)
  }
}

/// The variables and snippets a build substitutes into documents.
#[derive(Debug, Clone, Default)]
pub struct Library {
  pub variables: HashMap<String, String>,
  pub snippets: HashMap<String, Document>,
}

/// The node types that stand for library entries; the node's `key` attribute names the entry.
pub const VARIABLE_NODE: &str = "variable";
pub const SNIPPET_NODE: &str = "snippet";

/// How deep snippets may include snippets; saving refuses cycles, this only bounds corrupt data.
const MAX_DEPTH: usize = 8;

fn key_of(node: &Node) -> Option<&str> {
  node.attrs.as_ref()?.get("key")?.as_str()
}

/// The keys of the variables and snippets a document refers to directly, sorted.
pub fn entry_refs(doc: &Document) -> (BTreeSet<String>, BTreeSet<String>) {
  let (mut variables, mut snippets) = (BTreeSet::new(), BTreeSet::new());

  for node in doc.0.walk() {
    match (node.kind.as_str(), key_of(node)) {
      (VARIABLE_NODE, Some(key)) => variables.insert(key.to_string()),
      (SNIPPET_NODE, Some(key)) => snippets.insert(key.to_string()),
      _ => false,
    };
  }

  (variables, snippets)
}

/// The document with every variable replaced by its text and every snippet by its content, and the keys that have no entry
/// (those nodes are left out). Nothing that follows needs to know the library exists.
pub fn expand(doc: &Document, library: &Library) -> (Document, Vec<String>) {
  let mut missing = BTreeSet::new();
  let root = expand_node(&doc.0, library, 0, &mut missing);

  (Document(root), missing.into_iter().collect())
}

fn expand_node(node: &Node, library: &Library, depth: usize, missing: &mut BTreeSet<String>) -> Node {
  let mut out = node.clone();
  out.content = node.content.as_ref().map(|children| expand_children(children, library, depth, missing));

  out
}

fn expand_children(children: &[Node], library: &Library, depth: usize, missing: &mut BTreeSet<String>) -> Vec<Node> {
  let mut out = Vec::with_capacity(children.len());

  for child in children {
    match (child.kind.as_str(), key_of(child)) {
      (VARIABLE_NODE, Some(key)) => match library.variables.get(key) {
        Some(value) if !value.is_empty() => out.push(Node {
          kind: "text".into(),
          attrs: None,
          content: None,
          marks: child.marks.clone().map(|m: Vec<Mark>| m),
          text: Some(value.clone()),
        }),
        Some(_) => {}
        None => {
          missing.insert(format!("variable {key}"));
        }
      },
      (SNIPPET_NODE, Some(key)) => match library.snippets.get(key) {
        Some(snippet) if depth < MAX_DEPTH => {
          let inner = snippet.0.content.as_deref().unwrap_or_default();
          out.extend(expand_children(inner, library, depth + 1, missing));
        }
        Some(_) => {}
        None => {
          missing.insert(format!("snippet {key}"));
        }
      },
      _ => out.push(expand_node(child, library, depth, missing)),
    }
  }

  out
}

/// Whether adding or changing the snippet `key` to `doc` would make a snippet include itself, directly or through others.
/// Returns the chain (`a → b → a`) when it would.
pub fn snippet_cycle(key: &str, doc: &Document, snippets: &HashMap<String, Document>) -> Option<Vec<String>> {
  fn walk(target: &str, doc: &Document, snippets: &HashMap<String, Document>, path: &mut Vec<String>) -> bool {
    for next in entry_refs(doc).1 {
      if path.contains(&next) && next != target {
        continue;
      }
      path.push(next.clone());
      if next == target {
        return true;
      }
      if let Some(inner) = snippets.get(&next)
        && walk(target, inner, snippets, path)
      {
        return true;
      }
      path.pop();
    }
    false
  }

  let mut path = vec![key.to_string()];

  walk(key, doc, snippets, &mut path).then_some(path)
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  fn doc(content: serde_json::Value) -> Document {
    serde_json::from_value(json!({ "type": "doc", "content": content })).unwrap()
  }

  fn text(d: &Document) -> String {
    d.0.walk().iter().filter_map(|n| n.text.clone()).collect::<Vec<_>>().join("|")
  }

  #[test]
  fn variables_and_nested_snippets_are_expanded_and_missing_ones_reported() {
    let library = Library {
      variables: HashMap::from([("version".into(), "2.4".into())]),
      snippets: HashMap::from([
        (
          "outer".into(),
          doc(
            json!([{ "type": "paragraph", "content": [{ "type": "text", "text": "outer " }, { "type": "variable", "attrs": { "key": "version" } }] }, { "type": "snippet", "attrs": { "key": "inner" } }]),
          ),
        ),
        ("inner".into(), doc(json!([{ "type": "paragraph", "content": [{ "type": "text", "text": "inner" }] }]))),
      ]),
    };
    let page = doc(json!([
      { "type": "paragraph", "content": [{ "type": "text", "text": "v" }, { "type": "variable", "attrs": { "key": "version" }, "marks": [{ "type": "bold" }] }, { "type": "variable", "attrs": { "key": "nope" } }] },
      { "type": "snippet", "attrs": { "key": "outer" } },
      { "type": "snippet", "attrs": { "key": "gone" } },
    ]));

    let (expanded, missing) = expand(&page, &library);

    assert_eq!(text(&expanded), "v|2.4|outer |2.4|inner");
    assert_eq!(expanded.0.walk().iter().find(|n| n.text.as_deref() == Some("2.4")).unwrap().marks.as_ref().unwrap()[0].kind, "bold");
    assert_eq!(missing, ["snippet gone", "variable nope"]);
    assert_eq!(entry_refs(&page).1.into_iter().collect::<Vec<_>>(), ["gone", "outer"]);
  }

  #[test]
  fn a_snippet_that_includes_itself_is_found() {
    let include = |key: &str| doc(json!([{ "type": "snippet", "attrs": { "key": key } }]));
    let snippets = HashMap::from([
      ("a".to_string(), include("b")),
      ("b".to_string(), include("c")),
      ("c".to_string(), doc(json!([{ "type": "paragraph" }]))),
    ]);

    // Directly, and through a chain.
    assert_eq!(snippet_cycle("x", &include("x"), &snippets), Some(vec!["x".into(), "x".into()]));
    assert_eq!(snippet_cycle("c", &include("a"), &snippets), Some(vec!["c".into(), "a".into(), "b".into(), "c".into()]));
    // A snippet that merely uses others is fine, and so is one used twice.
    assert_eq!(snippet_cycle("z", &include("a"), &snippets), None);
  }
}
