use std::collections::HashMap;

use serde_json::{Value, json};
use yourdocs_core::{AssetId, Document, Page, PageId, header_pages, in_sidebar, sidebar_root};

/// Where the file for a page URL goes: `/a/b/` is `a/b/index.html`.
pub(crate) fn output_path(url: &str) -> String {
  format!("{}index.html", url.trim_start_matches('/'))
}

/// The sidebar for `current` (`None` for pages outside the tree, like the 404 page), in the order of `pages` (which is by position).
/// A page inside a promoted section sees only that section; other pages see the rest of the site. Links point outside the site.
pub(crate) fn nav_tree(pages: &[Page], urls: &HashMap<PageId, String>, current: Option<PageId>) -> Vec<Value> {
  fn children(pages: &[Page], urls: &HashMap<PageId, String>, root: Option<PageId>, parent: Option<PageId>) -> Vec<Value> {
    pages
      .iter()
      .filter(|p| p.parent_id == parent && in_sidebar(pages, root, p))
      .filter_map(|p| {
        let (href, external) = match &p.external_url {
          Some(url) => (url.as_str(), true),
          None => (urls.get(&p.id)?.as_str(), false),
        };

        Some(json!({ "id": p.id, "title": p.title, "href": href, "external": external, "children": children(pages, urls, root, Some(p.id)) }))
      })
      .collect()
  }

  let root = sidebar_root(pages, current);
  children(pages, urls, root, root)
}

/// The header entries: the promoted pages with their URLs.
pub(crate) fn header_items(pages: &[Page], urls: &HashMap<PageId, String>) -> Vec<Value> {
  header_pages(pages)
    .into_iter()
    .filter_map(|p| Some(json!({ "id": p.id, "title": p.title, "href": urls.get(&p.id)? })))
    .collect()
}

/// The assets an image in the document refers to.
pub(crate) fn image_asset_ids(doc: &Document) -> Vec<AssetId> {
  doc
    .0
    .walk()
    .into_iter()
    .filter(|node| node.kind == "image")
    .filter_map(|node| node.attrs.as_ref()?.get("assetId")?.as_str()?.parse().ok())
    .collect()
}

#[cfg(test)]
mod tests {
  use jiff::Timestamp;
  use yourdocs_core::{PageKind, UserId, page_urls};

  use super::*;

  fn page(slug: &str, parent: Option<PageId>) -> Page {
    Page {
      id: PageId::new(),
      kind: PageKind::Page,
      parent_id: parent,
      position: "80".into(),
      slug: slug.into(),
      title: slug.into(),
      external_url: None,
      path_override: None,
      in_header: false,
      header_position: None,
      created_by: UserId::new(),
      created_at: Timestamp::UNIX_EPOCH,
      updated_at: Timestamp::UNIX_EPOCH,
    }
  }

  #[test]
  fn urls_follow_the_tree_and_map_to_files() {
    let guide = page("guide", None);
    let install = page("install", Some(guide.id));
    let pages = vec![guide.clone(), install.clone()];
    let urls = page_urls(&pages);

    assert_eq!(urls[&guide.id], "/guide/");
    assert_eq!(urls[&install.id], "/guide/install/");
    assert_eq!(output_path("/guide/install/"), "guide/install/index.html");

    let nav = nav_tree(&pages, &urls, Some(install.id));
    assert_eq!(nav[0]["children"][0]["href"], "/guide/install/");
  }

  #[test]
  fn links_appear_in_the_navigation_but_have_no_url() {
    let guide = page("guide", None);
    let link = Page {
      kind: PageKind::Link,
      external_url: Some("https://status.example.com".into()),
      ..page("status", Some(guide.id))
    };
    let pages = vec![guide.clone(), link.clone()];
    let urls = page_urls(&pages);

    assert!(!urls.contains_key(&link.id));
    let nav = nav_tree(&pages, &urls, None);
    assert_eq!(nav[0]["children"][0]["href"], "https://status.example.com");
    assert_eq!(nav[0]["children"][0]["external"], true);
  }

  #[test]
  fn finds_images_in_a_document() {
    let id = AssetId::new();
    let doc: Document = serde_json::from_value(json!({ "type": "doc", "content": [
      { "type": "paragraph" },
      { "type": "tabs", "content": [{ "type": "tabPanel", "content": [{ "type": "image", "attrs": { "assetId": id.to_string() } }] }] },
    ] }))
    .unwrap();

    assert_eq!(image_asset_ids(&doc), [id]);
  }
}
