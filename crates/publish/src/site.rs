use std::collections::{HashMap, HashSet};

use cairn_core::{AssetId, Document, Page, PageId, header_pages, in_sidebar, sidebar_root};
use serde_json::{Value, json};

/// Where the file for a page URL goes: `/a/b/` is `a/b/index.html`.
pub(crate) fn output_path(url: &str) -> String {
  format!("{}index.html", url.trim_start_matches('/'))
}

/// The sidebar for `current` (`None` for pages outside the tree, like the 404 page), in the order of `pages` (which is by position).
/// A page inside a promoted section sees only that section; other pages see the rest of the site. Links point outside the site.
pub(crate) fn nav_tree(pages: &[Page], urls: &HashMap<PageId, String>, betas: &HashSet<PageId>, current: Option<PageId>) -> Vec<Value> {
  fn children(pages: &[Page], urls: &HashMap<PageId, String>, betas: &HashSet<PageId>, root: Option<PageId>, parent: Option<PageId>) -> Vec<Value> {
    pages
      .iter()
      .filter(|p| p.parent_id == parent && in_sidebar(pages, root, p))
      .filter_map(|p| {
        let (href, external) = match &p.external_url {
          Some(url) => (url.as_str(), true),
          None => (urls.get(&p.id)?.as_str(), false),
        };

        let nodes = children(pages, urls, betas, root, Some(p.id));

        let mut node = json!({ "id": p.id, "title": p.title, "href": href, "external": external, "children": nodes });
        // A pill after the title, for API references marked as beta.
        if betas.contains(&p.id) {
          node["badge"] = json!("Beta");
        }

        Some(node)
      })
      .collect()
  }

  let root = sidebar_root(pages, current);
  children(pages, urls, betas, root, root)
}

/// Where the back link at the top of a section's own sidebar leads, as `{ title, href }`: the page the section sits under, or for a
/// section at the top of the tree, the main documentation (its first page, titled with the site name). `None` when `current` is not in
/// such a sidebar, or the main documentation has no page to go back to.
pub(crate) fn section_back(pages: &[Page], urls: &HashMap<PageId, String>, current: PageId, site_name: &str) -> Option<Value> {
  fn first_page(nodes: &[Value]) -> Option<&Value> {
    nodes.iter().find_map(|n| if n["external"] == true { first_page(n["children"].as_array()?) } else { Some(&n["href"]) })
  }

  let section = sidebar_root(pages, Some(current))?;
  let parent = pages.iter().find(|p| p.id == section)?.parent_id;

  match parent.and_then(|id| pages.iter().find(|p| p.id == id)) {
    Some(parent) => Some(json!({ "title": parent.title, "href": urls.get(&parent.id)? })),
    None => Some(json!({ "title": site_name, "href": first_page(&nav_tree(pages, urls, &HashSet::new(), None))? })),
  }
}

/// The pages above `page`, outermost first, as `{ title, href }` for the breadcrumb trail. An ancestor without a URL is skipped.
pub(crate) fn breadcrumbs(pages: &[Page], urls: &HashMap<PageId, String>, page: PageId) -> Vec<Value> {
  let mut trail = Vec::new();
  let mut current = pages.iter().find(|p| p.id == page).and_then(|p| p.parent_id);

  // Bounded like the other tree walks, in case the data is corrupt.
  for _ in 0..64 {
    let Some(ancestor) = current.and_then(|id| pages.iter().find(|p| p.id == id)) else { break };
    if let Some(href) = urls.get(&ancestor.id) {
      trail.push(json!({ "title": ancestor.title, "href": href }));
    }
    current = ancestor.parent_id;
  }

  trail.reverse();
  trail
}

/// The pages before and after `page` in the order of the sidebar it appears in (`nav`, as built by `nav_tree`). Links to other sites are skipped.
pub(crate) fn previous_next(nav: &[Value], page: PageId) -> (Option<Value>, Option<Value>) {
  fn flatten(nodes: &[Value], into: &mut Vec<Value>) {
    for node in nodes {
      if node["external"] != true {
        into.push(json!({ "id": node["id"], "title": node["title"], "href": node["href"] }));
      }
      if let Some(children) = node["children"].as_array() {
        flatten(children, into);
      }
    }
  }

  let mut flat = Vec::new();
  flatten(nav, &mut flat);
  let id = json!(page);
  let Some(at) = flat.iter().position(|n| n["id"] == id) else { return (None, None) };
  let card = |n: &Value| json!({ "title": n["title"], "href": n["href"] });

  (at.checked_sub(1).map(|i| card(&flat[i])), flat.get(at + 1).map(card))
}

/// Where a page's Markdown file goes: `/guide/install/` is `guide/install.md`.
pub(crate) fn markdown_path(url: &str) -> String {
  format!("{}.md", url.trim_matches('/'))
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
  use cairn_core::{PageKind, UserId, page_urls};
  use jiff::Timestamp;

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
      description: None,
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

    let nav = nav_tree(&pages, &urls, &HashSet::new(), Some(install.id));
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
    let nav = nav_tree(&pages, &urls, &HashSet::new(), None);
    assert_eq!(nav[0]["children"][0]["href"], "https://status.example.com");
    assert_eq!(nav[0]["children"][0]["external"], true);
  }

  #[test]
  fn breadcrumbs_neighbours_and_markdown_paths() {
    let guide = page("guide", None);
    let install = page("install", Some(guide.id));
    let linux = page("linux", Some(install.id));
    let faq = page("faq", Some(guide.id));
    let pages = vec![guide.clone(), install.clone(), linux.clone(), faq.clone()];
    let urls = page_urls(&pages);

    let trail = breadcrumbs(&pages, &urls, linux.id);
    assert_eq!(trail.iter().map(|c| c["title"].as_str().unwrap()).collect::<Vec<_>>(), ["guide", "install"]);
    assert_eq!(trail[1]["href"], "/guide/install/");

    // Order is the tree's, depth first: guide, install, linux, faq.
    let nav = nav_tree(&pages, &urls, &HashSet::new(), Some(linux.id));
    let (previous, next) = previous_next(&nav, linux.id);
    assert_eq!((previous.unwrap()["title"].as_str(), next.unwrap()["title"].as_str()), (Some("install"), Some("faq")));
    assert_eq!(previous_next(&nav, guide.id).0, None);
    assert_eq!(previous_next(&nav, faq.id).1, None);

    assert_eq!(markdown_path("/guide/install/"), "guide/install.md");
  }

  #[test]
  fn a_section_sidebar_links_back_to_where_the_section_sits() {
    let promoted = |p: Page| Page {
      in_header: true,
      header_position: Some("80".into()),
      ..p
    };
    let guide = page("guide", None);
    let api = promoted(page("api", None));
    let endpoints = page("endpoints", Some(api.id));
    let sdks = promoted(page("sdks", Some(guide.id)));
    let python = page("python", Some(sdks.id));
    let lonely = promoted(page("changelog", None));
    let pages = vec![guide.clone(), api.clone(), endpoints.clone(), sdks.clone(), python.clone(), lonely.clone()];
    let urls = page_urls(&pages);
    let back = |id: PageId| section_back(&pages, &urls, id, "Docs").map(|b| (b["title"].as_str().unwrap().to_string(), b["href"].as_str().unwrap().to_string()));

    let cases = [
      // A top-level section goes back to the main documentation, named after the site.
      (endpoints.id, Some(("Docs", "/guide/"))),
      (api.id, Some(("Docs", "/guide/"))),
      // A section under a page goes back to that page.
      (python.id, Some(("guide", "/guide/"))),
      // Pages of the main documentation, and a promoted page without subpages, have the whole sidebar already.
      (guide.id, None),
      (lonely.id, None),
    ];
    for (id, expected) in cases {
      assert_eq!(back(id), expected.map(|(t, h)| (t.to_string(), h.to_string())), "{id}");
    }
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
