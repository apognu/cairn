use crate::{Page, PageId};

/// The pages promoted to the header, in header order.
pub fn header_pages(pages: &[Page]) -> Vec<&Page> {
  let mut promoted: Vec<&Page> = pages.iter().filter(|p| p.in_header && p.kind.has_url()).collect();
  promoted.sort_by(|a, b| a.header_position.cmp(&b.header_position));
  promoted
}

fn has_children(pages: &[Page], id: PageId) -> bool {
  pages.iter().any(|p| p.parent_id == Some(id))
}

/// The header entry a page belongs to: the nearest promoted page above it, or the page itself.
pub fn header_section(pages: &[Page], page: PageId) -> Option<PageId> {
  let mut current = pages.iter().find(|p| p.id == page);

  // Bounded like the other tree walks, in case the data is corrupt.
  for _ in 0..64 {
    let p = current?;
    if p.in_header && p.kind.has_url() {
      return Some(p.id);
    }
    current = pages.iter().find(|q| Some(q.id) == p.parent_id);
  }

  None
}

/// Which pages the sidebar of `current` lists, as the parent whose children form its top level.
/// `Some(id)` for a page inside a promoted section that has subpages; `None` for the rest of the site, which
/// leaves out those sections and the pages promoted without subpages (they live in the header).
pub fn sidebar_root(pages: &[Page], current: Option<PageId>) -> Option<PageId> {
  let section = header_section(pages, current?)?;

  has_children(pages, section).then_some(section)
}

/// Whether `page` is listed in the sidebar whose top level is `root`.
pub fn in_sidebar(pages: &[Page], root: Option<PageId>, page: &Page) -> bool {
  match root {
    Some(section) => page.id != section && is_below(pages, page, section),
    None => header_section(pages, page.id).is_none(),
  }
}

/// Whether `page` has `ancestor` somewhere above it.
fn is_below(pages: &[Page], page: &Page, ancestor: PageId) -> bool {
  let mut current = page.parent_id;

  for _ in 0..64 {
    let Some(id) = current else { return false };
    if id == ancestor {
      return true;
    }
    current = pages.iter().find(|p| p.id == id).and_then(|p| p.parent_id);
  }

  false
}

#[cfg(test)]
mod tests {
  use jiff::Timestamp;

  use super::*;
  use crate::{PageKind, UserId};

  fn page(slug: &str, parent: Option<&Page>, in_header: bool) -> Page {
    Page {
      id: PageId::new(),
      kind: PageKind::Page,
      parent_id: parent.map(|p| p.id),
      position: "80".into(),
      slug: slug.into(),
      title: slug.into(),
      external_url: None,
      path_override: None,
      in_header,
      header_position: in_header.then(|| "80".into()),
      description: None,
      created_by: UserId::new(),
      created_at: Timestamp::UNIX_EPOCH,
      updated_at: Timestamp::UNIX_EPOCH,
    }
  }

  #[test]
  fn a_promoted_section_scopes_the_sidebar_and_leaves_the_rest() {
    let guides = page("guides", None, true);
    let install = page("install", Some(&guides), false);
    let deep = page("deep", Some(&install), false);
    let cli = page("cli", None, true);
    let about = page("about", None, false);
    let all = [guides.clone(), install.clone(), deep.clone(), cli.clone(), about.clone()];

    assert_eq!(sidebar_root(&all, Some(deep.id)), Some(guides.id));
    assert_eq!(sidebar_root(&all, Some(guides.id)), Some(guides.id));
    // A promoted page without subpages is a plain header link; its sidebar is the rest of the site.
    assert_eq!(sidebar_root(&all, Some(cli.id)), None);
    assert_eq!(sidebar_root(&all, Some(about.id)), None);

    let listed = |root: Option<PageId>| all.iter().filter(|p| in_sidebar(&all, root, p)).map(|p| p.slug.as_str()).collect::<Vec<_>>();
    assert_eq!(listed(Some(guides.id)), ["install", "deep"]);
    assert_eq!(listed(None), ["about"]);
    assert_eq!(header_pages(&all).iter().map(|p| p.slug.as_str()).collect::<Vec<_>>(), ["guides", "cli"]);
    assert_eq!(header_section(&all, deep.id), Some(guides.id));
    assert_eq!(header_section(&all, about.id), None);
  }
}
