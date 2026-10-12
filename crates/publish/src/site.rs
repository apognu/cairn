use std::collections::HashMap;

use serde_json::{Value, json};
use yourdocs_core::{AssetId, Document, Page, PageId};

/// The public URL of every page: the slugs of the page and its ancestors, with a trailing slash.
pub(crate) fn page_urls(pages: &[Page]) -> HashMap<PageId, String> {
  let by_id: HashMap<PageId, &Page> = pages.iter().map(|p| (p.id, p)).collect();

  pages
    .iter()
    .map(|page| {
      let mut segments = vec![page.slug.as_str()];
      let mut parent = page.parent_id;

      // The tree has no cycles (moves are checked), but a bound keeps corrupt data from hanging a publish.
      while let Some(id) = parent
        && segments.len() < 64
        && let Some(ancestor) = by_id.get(&id)
      {
        segments.push(ancestor.slug.as_str());
        parent = ancestor.parent_id;
      }

      segments.reverse();
      (page.id, format!("/{}/", segments.join("/")))
    })
    .collect()
}

/// Where the file for a page URL goes: `/a/b/` is `a/b/index.html`.
pub(crate) fn output_path(url: &str) -> String {
  format!("{}index.html", url.trim_start_matches('/'))
}

/// The navigation tree for the page context, in the order of `pages` (which is by position).
pub(crate) fn nav_tree(pages: &[Page], urls: &HashMap<PageId, String>) -> Vec<Value> {
  fn children(pages: &[Page], urls: &HashMap<PageId, String>, parent: Option<PageId>) -> Vec<Value> {
    pages
      .iter()
      .filter(|p| p.parent_id == parent)
      .map(|p| json!({ "id": p.id, "title": p.title, "href": urls[&p.id], "children": children(pages, urls, Some(p.id)) }))
      .collect()
  }

  children(pages, urls, None)
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
  use yourdocs_core::UserId;

  use super::*;

  fn page(slug: &str, parent: Option<PageId>) -> Page {
    Page {
      id: PageId::new(),
      parent_id: parent,
      position: "80".into(),
      slug: slug.into(),
      title: slug.into(),
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

    let nav = nav_tree(&pages, &urls);
    assert_eq!(nav[0]["children"][0]["href"], "/guide/install/");
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
