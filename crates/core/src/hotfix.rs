//! A hotfix: the live site plus a few pages, with what those pages need to fit in the tree.

use std::collections::HashSet;

use serde::Serialize;

use crate::{Page, PageId};

/// Why a page that was not picked goes out with a hotfix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum PullReason {
  /// A picked page now lives under it, and it is not on the live site where it is now.
  Parent,
  /// It sits under a page the hotfix deletes or moves away from it.
  Orphan,
}

/// A page the hotfix takes along without being asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Pulled {
  pub page_id: PageId,
  pub title: String,
  pub reason: PullReason,
}

/// The tree a hotfix publishes: every live page as it was published, except the picked ones, which are taken as they are now (or left
/// out, when they were deleted). A page whose parent would be missing pulls that parent in when it exists now, or else is pulled in itself
/// (it was deleted or moved with its old parent). Returns the tree and what was pulled in, in the order it was found.
pub fn hotfix_pages(live: &[Page], current: &[Page], picked: &HashSet<PageId>) -> (Vec<Page>, Vec<Pulled>) {
  let mut included = picked.clone();
  let mut pulled = Vec::new();

  loop {
    let tree: Vec<Page> = live
      .iter()
      .filter(|p| !included.contains(&p.id))
      .chain(current.iter().filter(|p| included.contains(&p.id)))
      .cloned()
      .collect();

    let missing_parent = tree
      .iter()
      .find_map(|p| p.parent_id.filter(|parent| !tree.iter().any(|t| t.id == *parent)).map(|parent| (p.id, parent)));
    let Some((page, parent)) = missing_parent else {
      return (tree, pulled);
    };

    let (id, reason) = match current.iter().find(|c| c.id == parent) {
      Some(_) if !included.contains(&parent) => (parent, PullReason::Parent),
      _ => (page, PullReason::Orphan),
    };
    // Every round adds a page, and the trees are finite; a page already in cannot come back, which would loop.
    if !included.insert(id) {
      return (tree.into_iter().filter(|p| p.id != page).collect(), pulled);
    }
    let title = current.iter().chain(live).find(|p| p.id == id).map(|p| p.title.clone()).unwrap_or_default();
    pulled.push(Pulled { page_id: id, title, reason });
  }
}

#[cfg(test)]
mod tests {
  use jiff::Timestamp;

  use super::*;
  use crate::{PageKind, UserId};

  fn page(id: PageId, parent: Option<PageId>, title: &str) -> Page {
    Page {
      id,
      kind: PageKind::Page,
      parent_id: parent,
      position: "a0".into(),
      slug: title.to_lowercase(),
      title: title.into(),
      external_url: None,
      path_override: None,
      in_header: false,
      hidden: false,
      header_position: None,
      description: None,
      created_by: UserId::new(),
      created_at: Timestamp::UNIX_EPOCH,
      updated_at: Timestamp::UNIX_EPOCH,
    }
  }

  #[test]
  fn a_hotfix_takes_the_picked_pages_and_what_they_need() {
    let [guide, install, setup, faq, legacy, child, new_section, new_page] = std::array::from_fn::<_, 8, _>(|_| PageId::new());
    let live = vec![
      page(guide, None, "Guide"),
      page(install, Some(guide), "Install"),
      page(setup, Some(guide), "Setup"),
      page(faq, None, "FAQ"),
      page(legacy, None, "Legacy"),
      page(child, Some(legacy), "Child"),
    ];
    // Now: Install is renamed, Setup moved under a new section, FAQ has a new subpage, Legacy and its child are deleted.
    let current = vec![
      page(guide, None, "Guide"),
      page(install, Some(guide), "Installing"),
      page(new_section, None, "Section"),
      page(setup, Some(new_section), "Setup"),
      page(faq, None, "FAQ"),
      page(new_page, Some(faq), "New"),
    ];
    let titles = |tree: &[Page]| {
      let mut t: Vec<String> = tree.iter().map(|p| p.title.clone()).collect();
      t.sort();
      t
    };

    // (picked, the tree's titles, what was pulled in)
    type Case<'a> = (Vec<PageId>, Vec<&'a str>, Vec<(PageId, PullReason)>);
    let cases: Vec<Case> = vec![
      (vec![install], vec!["Child", "FAQ", "Guide", "Installing", "Legacy", "Setup"], vec![]),
      (
        vec![setup],
        vec!["Child", "FAQ", "Guide", "Install", "Legacy", "Section", "Setup"],
        vec![(new_section, PullReason::Parent)],
      ),
      (vec![new_page], vec!["Child", "FAQ", "Guide", "Install", "Legacy", "New", "Setup"], vec![]),
      (vec![legacy], vec!["FAQ", "Guide", "Install", "Setup"], vec![(child, PullReason::Orphan)]),
      (vec![legacy, child], vec!["FAQ", "Guide", "Install", "Setup"], vec![]),
      (vec![guide], vec!["Child", "FAQ", "Guide", "Install", "Legacy", "Setup"], vec![]),
    ];

    for (picked, expected, expected_pulled) in cases {
      let (tree, pulled) = hotfix_pages(&live, &current, &picked.iter().copied().collect());
      assert_eq!(titles(&tree), expected, "{picked:?}");
      assert_eq!(pulled.iter().map(|p| (p.page_id, p.reason)).collect::<Vec<_>>(), expected_pulled, "{picked:?}");
    }
  }
}
