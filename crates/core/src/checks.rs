use std::collections::HashMap;

use serde::Serialize;

use crate::{Document, Page, PageId, extract_links, page_urls};

/// What a pre-publish check found wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ProblemKind {
  /// A link to a page that no longer exists.
  BrokenLink,
  /// A component whose settings do not fit its schema, or a block the renderer does not know.
  InvalidProps,
  /// Two pages with the same URL.
  DuplicateUrl,
  /// An image with no description and not marked decorative.
  MissingAlt,
}

/// A problem that stops a publish, with the page it is on.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Problem {
  pub page_id: PageId,
  pub page_title: String,
  pub kind: ProblemKind,
  pub message: String,
}

impl std::fmt::Display for Problem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "page \"{}\": {}", self.page_title, self.message)
  }
}

fn problem(page: &Page, kind: ProblemKind, message: impl Into<String>) -> Problem {
  Problem {
    page_id: page.id,
    page_title: page.title.clone(),
    kind,
    message: message.into(),
  }
}

/// Problems found in one page's document: links to pages that are gone (`pages` is every page that exists) and images without alt text.
pub fn doc_problems(page: &Page, doc: &Document, pages: &[Page]) -> Vec<Problem> {
  let mut found = Vec::new();

  let gone = extract_links(doc).iter().filter(|l| !pages.iter().any(|p| p.id == l.target && p.kind.has_url())).count();
  if gone > 0 {
    found.push(problem(
      page,
      ProblemKind::BrokenLink,
      format!("{} to a page that was deleted", if gone == 1 { "1 link points".to_string() } else { format!("{gone} links point") }),
    ));
  }

  let missing = doc
    .0
    .walk()
    .into_iter()
    .filter(|n| n.kind == "image")
    .filter(|n| {
      let attrs = n.attrs.as_ref();
      let alt = attrs.and_then(|a| a.get("alt")?.as_str()).unwrap_or_default();
      let decorative = attrs.and_then(|a| a.get("decorative")?.as_bool()).unwrap_or(false);

      alt.trim().is_empty() && !decorative
    })
    .count();
  if missing > 0 {
    found.push(problem(
      page,
      ProblemKind::MissingAlt,
      format!(
        "{} no alt text; describe {} or mark {} decorative",
        if missing == 1 { "1 image has".to_string() } else { format!("{missing} images have") },
        if missing == 1 { "it" } else { "them" },
        if missing == 1 { "it" } else { "them" }
      ),
    ));
  }

  found
}

/// Pages that would be published at the same URL, one problem per page involved.
pub fn url_problems(pages: &[Page]) -> Vec<Problem> {
  let urls = page_urls(pages);
  let mut by_url: HashMap<&str, Vec<&Page>> = HashMap::new();

  for page in pages {
    if let Some(url) = urls.get(&page.id) {
      by_url.entry(url).or_default().push(page);
    }
  }

  let mut found = Vec::new();
  for (url, group) in by_url.into_iter().filter(|(_, group)| group.len() > 1) {
    for page in &group {
      let others: Vec<String> = group.iter().filter(|p| p.id != page.id).map(|p| format!("“{}”", p.title)).collect();
      found.push(problem(page, ProblemKind::DuplicateUrl, format!("{url} is also the URL of {}", others.join(", "))));
    }
  }
  found.sort_by(|a, b| a.page_title.cmp(&b.page_title));

  found
}

#[cfg(test)]
mod tests {
  use jiff::Timestamp;
  use serde_json::json;

  use super::*;
  use crate::{PageKind, UserId};

  fn page(title: &str, slug: &str, custom: Option<&str>) -> Page {
    Page {
      id: PageId::new(),
      kind: PageKind::Page,
      parent_id: None,
      position: "80".into(),
      slug: slug.into(),
      title: title.into(),
      external_url: None,
      path_override: custom.map(Into::into),
      in_header: false,
      header_position: None,
      description: None,
      created_by: UserId::new(),
      created_at: Timestamp::UNIX_EPOCH,
      updated_at: Timestamp::UNIX_EPOCH,
    }
  }

  #[test]
  fn finds_broken_links_and_missing_alt_text() {
    let (home, other) = (page("Home", "home", None), page("Other", "other", None));
    let gone = PageId::new();
    let doc: Document = serde_json::from_value(json!({ "type": "doc", "content": [
      { "type": "paragraph", "content": [
        { "type": "text", "text": "ok", "marks": [{ "type": "link", "attrs": { "pageId": other.id.to_string() } }] },
        { "type": "text", "text": "dead", "marks": [{ "type": "link", "attrs": { "pageId": gone.to_string() } }] },
      ] },
      { "type": "image", "attrs": { "assetId": "x", "alt": "" } },
      { "type": "image", "attrs": { "assetId": "y", "alt": "  " } },
      { "type": "image", "attrs": { "assetId": "z", "alt": "", "decorative": true } },
      { "type": "image", "attrs": { "assetId": "w", "alt": "A chart" } },
    ] }))
    .unwrap();

    let found = doc_problems(&home, &doc, &[home.clone(), other]);
    let summary: Vec<_> = found.iter().map(|p| (p.kind, p.to_string())).collect();

    assert_eq!(
      summary,
      [
        (ProblemKind::BrokenLink, "page \"Home\": 1 link points to a page that was deleted".to_string()),
        (ProblemKind::MissingAlt, "page \"Home\": 2 images have no alt text; describe them or mark them decorative".to_string()),
      ]
    );
  }

  #[test]
  fn names_every_page_that_shares_a_url() {
    let (a, b, c) = (page("Alpha", "alpha", None), page("Beta", "beta", Some("/alpha/")), page("Gamma", "gamma", None));
    let found = url_problems(&[a, b, c]);

    assert_eq!(
      found.iter().map(|p| (p.page_title.as_str(), p.message.as_str())).collect::<Vec<_>>(),
      [("Alpha", "/alpha/ is also the URL of “Beta”"), ("Beta", "/alpha/ is also the URL of “Alpha”"),]
    );
  }
}
