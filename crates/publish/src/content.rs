use std::collections::{BTreeMap, HashMap};

use cairn_core::{Document, Library, Page, PageId, extract_links, term_refs};
use serde_json::json;
use sha2::{Digest, Sha256};

/// A hash of what a page's content is made of: its document with the library written in (`expand`), and what it shows of the pages it
/// links to and the terms it marks. Unlike the render cache key it leaves out the navigation, so renaming a page changes only the pages
/// that link to it. Recorded with each release page; a difference from the live one means the page changed without being edited.
pub(crate) fn content_hash(expanded: &Document, pages: &[Page], urls: &HashMap<PageId, String>, library: &Library) -> String {
  let linked: BTreeMap<String, _> = extract_links(expanded)
    .into_iter()
    .map(|link| {
      let page = pages.iter().find(|p| p.id == link.target);
      let shown = json!({ "title": page.map(|p| &p.title), "description": page.and_then(|p| p.description.as_ref()), "url": urls.get(&link.target) });
      (link.target.to_string(), shown)
    })
    .collect();
  let terms: BTreeMap<String, _> = term_refs(expanded)
    .into_iter()
    .map(|t| (t.clone(), library.terms.get(&t).map(|term| term.definition.plain_text())))
    .collect();

  let content = json!({ "doc": expanded, "pages": linked, "terms": terms });
  Sha256::digest(content.to_string()).iter().map(|b| format!("{b:02x}")).collect()
}
