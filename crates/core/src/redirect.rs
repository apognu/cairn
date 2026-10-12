use std::collections::{HashMap, HashSet};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{Page, PageId, PageKind, page_urls};

/// Whether a move or rename keeps the old URL working.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldUrls {
  Redirect,
  Forget,
}

/// Why a redirect exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RedirectOrigin {
  Move,
  Rename,
  Manual,
}

impl RedirectOrigin {
  pub fn as_str(self) -> &'static str {
    match self {
      RedirectOrigin::Move => "move",
      RedirectOrigin::Rename => "rename",
      RedirectOrigin::Manual => "manual",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    [Self::Move, Self::Rename, Self::Manual].into_iter().find(|o| o.as_str() == value)
  }
}

/// An old path that sends visitors to a page (which may have moved again since) or to a fixed URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Redirect {
  /// Lowercase, with leading and trailing slash (`/guide/install/`).
  pub from_path: String,
  pub to_page_id: Option<PageId>,
  pub to_url: Option<String>,
  pub origin: RedirectOrigin,
  #[cfg_attr(feature = "openapi", schema(value_type = String, format = DateTime))]
  pub created_at: Timestamp,
}

/// Why a redirect is not written, if it is not: a page now lives at its path, its page is gone, or it is part of a loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum Inactive {
  PageAtPath,
  TargetGone,
  Loop,
}

/// Where `redirect` leads on the built site, or why nowhere. Chains are followed to their end: with /a/ → /b/ and /b/ → /c/,
/// /a/ goes straight to /c/, so a visitor is redirected once.
pub fn resolve_redirect(redirect: &Redirect, redirects: &[Redirect], pages: &[Page]) -> Result<String, Inactive> {
  let urls = page_urls(pages);
  let taken: HashSet<&str> = urls.values().map(String::as_str).collect();

  let first_hop = |r: &Redirect| match (&r.to_page_id, &r.to_url) {
    (Some(id), _) => urls.get(id).cloned().ok_or(Inactive::TargetGone),
    (None, Some(url)) => Ok(url.clone()),
    (None, None) => Err(Inactive::TargetGone),
  };

  if taken.contains(redirect.from_path.as_str()) {
    return Err(Inactive::PageAtPath);
  }

  let mut at = first_hop(redirect)?;
  // A bound longer than any sane chain also catches loops.
  for _ in 0..16 {
    // A page at the path ends the chain, whatever redirect also starts there.
    if taken.contains(at.as_str()) {
      return Ok(at);
    }
    match redirects.iter().find(|r| r.from_path == at) {
      Some(next) if next.from_path == redirect.from_path => return Err(Inactive::Loop),
      Some(next) => at = first_hop(next)?,
      None => return Ok(at),
    }
  }

  Err(Inactive::Loop)
}

/// The redirects a build writes, as `(from, to)` pairs sorted by `from`, each pointing straight at its final destination.
/// Redirects to pages follow the page to wherever it lives now; the ones that cannot be resolved are left out.
pub fn effective_redirects(redirects: &[Redirect], pages: &[Page]) -> Vec<(String, String)> {
  let mut rules: Vec<(String, String)> = redirects
    .iter()
    .filter_map(|r| resolve_redirect(r, redirects, pages).ok().map(|to| (r.from_path.clone(), to)))
    .filter(|(from, to)| from != to)
    .collect();

  rules.sort();
  rules
}

/// What changed about a page since the live release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
  New,
  Edited,
  Renamed,
  Moved,
  /// Promoted to or taken out of the site header.
  Header,
  Deleted,
}

/// One page with something to publish.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Change {
  pub page_id: PageId,
  pub title: String,
  pub kinds: Vec<ChangeKind>,
  /// The URL in the live release, for pages that were already published.
  pub old_url: Option<String>,
  /// The URL after publishing, for pages that still exist.
  pub new_url: Option<String>,
  /// Whether the old URL keeps working through a redirect.
  pub redirected: bool,
}

/// A URL that works on the live site and will not after publishing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LostUrl {
  pub path: String,
  pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Pending {
  pub changes: Vec<Change>,
  pub lost_urls: Vec<LostUrl>,
}

/// The published state to compare against: the tree, the redirects and the revision of each page.
pub struct Live<'a> {
  pub pages: &'a [Page],
  pub redirects: &'a [(String, String)],
  pub revisions: &'a HashMap<PageId, crate::RevisionId>,
}

/// Compares the current tree and drafts with the live release. `drafts` holds each page's latest saved revision.
pub fn pending_changes(pages: &[Page], redirects: &[Redirect], drafts: &HashMap<PageId, crate::RevisionId>, live: Option<&Live>) -> Pending {
  let urls = page_urls(pages);
  let rules = effective_redirects(redirects, pages);
  let empty = Live {
    pages: &[],
    redirects: &[],
    revisions: &HashMap::new(),
  };
  let live = live.unwrap_or(&empty);
  let live_urls = page_urls(live.pages);

  let mut changes = Vec::new();

  for page in pages {
    let before = live.pages.iter().find(|p| p.id == page.id);
    let mut kinds = Vec::new();

    match before {
      None => kinds.push(ChangeKind::New),
      Some(old) => {
        if old.parent_id != page.parent_id {
          kinds.push(ChangeKind::Moved);
        }
        if old.slug != page.slug || old.title != page.title {
          kinds.push(ChangeKind::Renamed);
        }
        if old.in_header != page.in_header {
          kinds.push(ChangeKind::Header);
        }
        let edited = match page.kind {
          PageKind::Page => drafts.get(&page.id).is_some_and(|draft| live.revisions.get(&page.id) != Some(draft)),
          PageKind::Link => old.external_url != page.external_url,
        };
        if edited {
          kinds.push(ChangeKind::Edited);
        }
      }
    }

    if kinds.is_empty() {
      continue;
    }

    let old_url = live_urls.get(&page.id).cloned();
    let new_url = urls.get(&page.id).cloned();
    let redirected = old_url.as_ref().is_some_and(|old| old != new_url.as_ref().unwrap_or(old) && rules.iter().any(|(from, _)| from == old));

    changes.push(Change {
      page_id: page.id,
      title: page.title.clone(),
      kinds,
      old_url,
      new_url,
      redirected,
    });
  }

  for old in live.pages.iter().filter(|p| !pages.iter().any(|p2| p2.id == p.id)) {
    changes.push(Change {
      page_id: old.id,
      title: old.title.clone(),
      kinds: vec![ChangeKind::Deleted],
      old_url: live_urls.get(&old.id).cloned(),
      new_url: None,
      redirected: false,
    });
  }

  let now_served: HashSet<&str> = urls.values().map(String::as_str).chain(rules.iter().map(|(from, _)| from.as_str())).collect();
  let mut lost_urls: Vec<LostUrl> = live_urls
    .iter()
    .map(|(id, path)| (path, live.pages.iter().find(|p| p.id == *id).map(|p| p.title.clone()).unwrap_or_default()))
    .chain(live.redirects.iter().map(|(from, to)| (from, format!("redirect to {to}"))))
    .filter(|(path, _)| !now_served.contains(path.as_str()))
    .map(|(path, title)| LostUrl { path: path.clone(), title })
    .collect();
  lost_urls.sort_by(|a, b| a.path.cmp(&b.path));
  lost_urls.dedup_by(|a, b| a.path == b.path);

  Pending { changes, lost_urls }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{RevisionId, UserId};

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

  fn redirect(from: &str, to: &Page) -> Redirect {
    Redirect {
      from_path: from.into(),
      to_page_id: Some(to.id),
      to_url: None,
      origin: RedirectOrigin::Move,
      created_at: Timestamp::UNIX_EPOCH,
    }
  }

  #[test]
  fn a_page_at_a_path_beats_a_redirect_from_it() {
    let a = page("a", None);
    let rules = effective_redirects(&[redirect("/a/", &a), redirect("/old/", &a)], std::slice::from_ref(&a));

    assert_eq!(rules, [("/old/".to_string(), "/a/".to_string())]);
  }

  #[test]
  fn chains_are_collapsed_and_loops_dropped() {
    let c = page("c", None);
    let to_url = |from: &str, to: &str| Redirect {
      from_path: from.into(),
      to_page_id: None,
      to_url: Some(to.into()),
      origin: RedirectOrigin::Manual,
      created_at: Timestamp::UNIX_EPOCH,
    };

    // /a/ -> /b/ -> /c/ (a page) becomes /a/ -> /c/ directly.
    let chain = [to_url("/a/", "/b/"), to_url("/b/", "/c/")];
    let rules = effective_redirects(&chain, std::slice::from_ref(&c));
    assert_eq!(rules, [("/a/".to_string(), "/c/".to_string()), ("/b/".to_string(), "/c/".to_string())]);

    // Through a redirect to a page that moved.
    let moved = [to_url("/a/", "/b/"), redirect("/b/", &c)];
    assert_eq!(effective_redirects(&moved, std::slice::from_ref(&c))[0], ("/a/".to_string(), "/c/".to_string()));

    // A loop leads nowhere, so neither end is written.
    let looped = [to_url("/x/", "/y/"), to_url("/y/", "/x/")];
    assert!(effective_redirects(&looped, std::slice::from_ref(&c)).is_empty());
    assert_eq!(resolve_redirect(&looped[0], &looped, std::slice::from_ref(&c)), Err(Inactive::Loop));
  }

  #[test]
  fn flags_urls_that_stop_working() {
    let guide = page("guide", None);
    let live_install = page("install", Some(guide.id));
    let mut moved = live_install.clone();
    moved.parent_id = None;

    let revisions = HashMap::new();
    let live = Live {
      pages: &[guide.clone(), live_install.clone()],
      redirects: &[],
      revisions: &revisions,
    };
    let now = [guide.clone(), moved.clone()];

    // Moved without a redirect: the old URL is lost and the change says so.
    let without = pending_changes(&now, &[], &HashMap::new(), Some(&live));
    assert_eq!(
      without.lost_urls,
      [LostUrl {
        path: "/guide/install/".into(),
        title: "install".into()
      }]
    );
    assert_eq!(without.changes.len(), 1);
    assert_eq!(without.changes[0].kinds, [ChangeKind::Moved]);
    assert!(!without.changes[0].redirected);

    // With a redirect it keeps working.
    let with = pending_changes(&now, &[redirect("/guide/install/", &moved)], &HashMap::new(), Some(&live));
    assert!(with.lost_urls.is_empty());
    assert!(with.changes[0].redirected);
  }

  #[test]
  fn new_edited_and_deleted_pages_are_listed() {
    let kept = page("kept", None);
    let gone = page("gone", None);
    let fresh = page("fresh", None);
    let (r1, r2) = (RevisionId::new(), RevisionId::new());

    let revisions = HashMap::from([(kept.id, r1)]);
    let live = Live {
      pages: &[kept.clone(), gone.clone()],
      redirects: &[],
      revisions: &revisions,
    };

    // The same revision as live: only the missing page shows.
    let unchanged = pending_changes(std::slice::from_ref(&kept), &[], &HashMap::from([(kept.id, r1)]), Some(&live));
    assert_eq!(unchanged.changes.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["gone"]);

    let drafts = HashMap::from([(kept.id, r2)]);
    let result = pending_changes(&[kept.clone(), fresh.clone()], &[], &drafts, Some(&live));
    let kinds: Vec<_> = result.changes.iter().map(|c| (c.title.as_str(), c.kinds.clone())).collect();
    assert_eq!(kinds, [("kept", vec![ChangeKind::Edited]), ("fresh", vec![ChangeKind::New]), ("gone", vec![ChangeKind::Deleted])]);
  }
}
