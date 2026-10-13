use std::collections::HashMap;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{PageId, UserId};

/// What a tree node is: a page of the site or a link to somewhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum PageKind {
  Page,
  /// An entry of the navigation that points to an external URL; it has no content and no children.
  Link,
  /// An API reference built from OpenAPI specs at every publish; it has no body of its own, but it has a URL and may have children.
  Api,
}

impl PageKind {
  pub fn as_str(self) -> &'static str {
    match self {
      PageKind::Page => "page",
      PageKind::Link => "link",
      PageKind::Api => "api",
    }
  }

  /// Whether the node is a page of the site with a URL of its own (everything but links).
  pub fn has_url(self) -> bool {
    self != PageKind::Link
  }
}

/// A node of the page tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Page {
  pub id: PageId,
  pub kind: PageKind,
  pub parent_id: Option<PageId>,
  /// Fractional index; siblings sort by this string.
  pub position: String,
  /// Lowercase, unique among siblings.
  pub slug: String,
  pub title: String,
  /// Where a link goes; always `None` for pages.
  pub external_url: Option<String>,
  /// A fixed URL that replaces the one derived from the tree (`/webhooks/`); it stays put when the page moves.
  #[serde(default)]
  pub path_override: Option<String>,
  /// Shown as an entry of the site header; a page with children also gets a sidebar of its own.
  #[serde(default)]
  pub in_header: bool,
  /// Orders the header entries (fractional index, like `position`).
  #[serde(default)]
  pub header_position: Option<String>,
  /// A short summary, shown on link cards and in the page's meta description.
  #[serde(default)]
  pub description: Option<String>,
  pub created_by: UserId,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub created_at: Timestamp,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub updated_at: Timestamp,
}

/// Where a page goes among its new siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
  Last,
  /// Right before the given sibling.
  Before(PageId),
}

/// The public URL of every page (links have none), with a trailing slash. A page with a custom URL has that; any other page's URL
/// is its parent's followed by its own slug, so a custom URL carries the pages below it along.
pub fn page_urls(pages: &[Page]) -> HashMap<PageId, String> {
  let by_id: HashMap<PageId, &Page> = pages.iter().map(|p| (p.id, p)).collect();
  let mut urls = HashMap::new();

  for page in pages.iter().filter(|p| p.kind.has_url()) {
    // The tree has no cycles (moves are checked), but a bound keeps corrupt data from hanging a publish.
    let mut chain = vec![page];
    while let Some(parent) = chain.last().and_then(|p| p.parent_id).and_then(|id| by_id.get(&id))
      && chain.len() < 64
      && chain.last().is_some_and(|p| p.path_override.is_none())
    {
      chain.push(parent);
    }

    // Walk back down from the first page that has a URL of its own (or the root).
    let mut url = match chain.last().and_then(|p| p.path_override.clone()) {
      Some(custom) => {
        chain.pop();
        custom
      }
      None => "/".to_string(),
    };
    for ancestor in chain.iter().rev() {
      url = format!("{url}{}/", ancestor.slug);
    }
    urls.insert(page.id, url);
  }

  urls
}

/// Turns a title into a URL segment: lowercase ASCII letters and digits separated by single dashes.
pub fn slugify(title: &str) -> String {
  let mut slug = String::new();

  for c in title.chars() {
    if c.is_ascii_alphanumeric() {
      slug.push(c.to_ascii_lowercase());
    } else if !slug.is_empty() && !slug.ends_with('-') {
      slug.push('-');
    }
  }

  slug.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::UserId;

  fn node(slug: &str, parent: Option<&Page>, custom: Option<&str>) -> Page {
    Page {
      id: PageId::new(),
      kind: PageKind::Page,
      parent_id: parent.map(|p| p.id),
      position: "80".into(),
      slug: slug.into(),
      title: slug.into(),
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
  fn a_custom_url_replaces_the_derived_one_and_carries_its_children() {
    let guide = node("guide", None, None);
    let hooks = node("webhooks", Some(&guide), Some("/hooks/"));
    let retries = node("retries", Some(&hooks), None);
    let urls = page_urls(&[guide.clone(), hooks.clone(), retries.clone()]);

    assert_eq!(urls[&guide.id], "/guide/");
    assert_eq!(urls[&hooks.id], "/hooks/");
    assert_eq!(urls[&retries.id], "/hooks/retries/");
  }

  #[test]
  fn slugifies_titles() {
    for (title, slug) in [("Getting Started", "getting-started"), ("  What's new?  ", "what-s-new"), ("API v2", "api-v2"), ("???", "")] {
      assert_eq!(slugify(title), slug, "{title}");
    }
  }
}
