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
}

impl PageKind {
  pub fn as_str(self) -> &'static str {
    match self {
      PageKind::Page => "page",
      PageKind::Link => "link",
    }
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

  #[test]
  fn slugifies_titles() {
    for (title, slug) in [("Getting Started", "getting-started"), ("  What's new?  ", "what-s-new"), ("API v2", "api-v2"), ("???", "")] {
      assert_eq!(slugify(title), slug, "{title}");
    }
  }
}
