use std::collections::{HashMap, HashSet};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{ApiConfig, Page, PageId, PageKind, Settings, page_urls};

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
  /// The title in the live release, when it changed.
  pub old_title: Option<String>,
  /// The revision a publish would take, for pages with content.
  pub revision_id: Option<crate::RevisionId>,
  /// The revision the live site shows, to compare with.
  pub live_revision_id: Option<crate::RevisionId>,
}

/// A URL that works on the live site and will not after publishing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LostUrl {
  pub path: String,
  pub title: String,
}

/// A link from a page that stays to a page that is going away.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BrokenLink {
  pub source_id: PageId,
  pub source_title: String,
  pub target_title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Pending {
  pub changes: Vec<Change>,
  pub lost_urls: Vec<LostUrl>,
  /// Links that would point at a deleted page: they render as plain text.
  pub broken_links: Vec<BrokenLink>,
  /// The site settings (name, logos, theme, header links, footer, custom CSS) differ from the live release's.
  pub settings: bool,
}

/// The published state to compare against: the tree, the redirects, the revision and content hash of each page and the settings it was
/// built with. A release made before settings were recorded has none, and is taken to have the current ones; one made before content
/// hashes has none either, and only revisions are compared.
pub struct Live<'a> {
  pub pages: &'a [Page],
  pub redirects: &'a [(String, String)],
  pub revisions: &'a HashMap<PageId, crate::RevisionId>,
  pub content: &'a HashMap<PageId, String>,
  pub settings: Option<&'a Settings>,
  pub api_configs: &'a HashMap<PageId, ApiConfig>,
}

/// The state a publish would take. `drafts` holds each page's latest saved revision; `content` a hash of what each page's content is made
/// of now (its expanded document and what it shows of the pages and terms it uses), as the publisher records it; `links` the page links;
/// `api_configs` the saved settings of each API page that has any.
pub struct Current<'a> {
  pub pages: &'a [Page],
  pub redirects: &'a [Redirect],
  pub drafts: &'a HashMap<PageId, crate::RevisionId>,
  pub content: &'a HashMap<PageId, String>,
  pub links: &'a [(PageId, PageId)],
  pub api_configs: &'a HashMap<PageId, ApiConfig>,
  pub settings: &'a Settings,
}

/// Compares the current tree, drafts, content and settings with the live release.
pub fn pending_changes(current: &Current, live: Option<&Live>) -> Pending {
  let Current {
    pages,
    redirects,
    drafts,
    content,
    links,
    api_configs,
    settings,
  } = *current;
  let urls = page_urls(pages);
  let rules = effective_redirects(redirects, pages);
  let no_configs = HashMap::new();
  let no_content = HashMap::new();
  let empty = Live {
    pages: &[],
    redirects: &[],
    revisions: &HashMap::new(),
    content: &no_content,
    settings: None,
    api_configs: &no_configs,
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
          // The content hash also changes when a variable, snippet or term the page uses changes, or a page it links to is renamed.
          PageKind::Page => {
            drafts.get(&page.id).is_some_and(|draft| live.revisions.get(&page.id) != Some(draft))
              || old.description != page.description
              || live.content.get(&page.id).is_some_and(|old| content.get(&page.id).is_some_and(|new| new != old))
          }
          PageKind::Link => old.external_url != page.external_url,
          // Spec updates are never edits: they arrive with every build. Its own settings are, compared with what the live release recorded.
          PageKind::Api => live.api_configs.get(&page.id).is_some_and(|old| old != api_configs.get(&page.id).unwrap_or(&ApiConfig::default())),
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
      old_title: before.filter(|old| old.title != page.title).map(|old| old.title.clone()),
      kinds,
      old_url,
      new_url,
      redirected,
      revision_id: drafts.get(&page.id).copied(),
      live_revision_id: live.revisions.get(&page.id).copied(),
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
      old_title: None,
      revision_id: None,
      live_revision_id: live.revisions.get(&old.id).copied(),
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

  let broken_links = links
    .iter()
    .filter_map(|(source, target)| {
      let source_page = pages.iter().find(|p| p.id == *source)?;
      if pages.iter().any(|p| p.id == *target) {
        return None;
      }
      let target_title = live.pages.iter().find(|p| p.id == *target).map_or_else(|| "a deleted page".to_string(), |p| p.title.clone());

      Some(BrokenLink {
        source_id: *source,
        source_title: source_page.title.clone(),
        target_title,
      })
    })
    .collect();

  Pending {
    changes,
    lost_urls,
    broken_links,
    settings: live.settings.is_some_and(|old| old != settings),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{RevisionId, UserId};

  /// `Current` with only a tree, redirects, drafts and links: no content hashes, API pages or settings to compare.
  fn pending_for(pages: &[Page], redirects: &[Redirect], drafts: &HashMap<PageId, crate::RevisionId>, links: &[(PageId, PageId)], live: &Live) -> Pending {
    let current = Current {
      pages,
      redirects,
      drafts,
      content: &HashMap::new(),
      links,
      api_configs: &HashMap::new(),
      settings: &Settings::default(),
    };

    pending_changes(&current, Some(live))
  }

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
  fn api_page_settings_and_site_settings_are_pending_until_published() {
    let mut api = page("orders", None);
    api.kind = PageKind::Api;
    let published = ApiConfig::default();
    let saved = ApiConfig {
      display: crate::ApiDisplay { beta: true, ..Default::default() },
      ..Default::default()
    };
    let site = Settings::default();
    let renamed = Settings {
      site_name: "Halyard".into(),
      ..Settings::default()
    };

    let revisions = HashMap::new();
    let recorded = HashMap::from([(api.id, published.clone())]);
    let live = Live {
      pages: std::slice::from_ref(&api),
      redirects: &[],
      revisions: &revisions,
      content: &HashMap::new(),
      settings: Some(&site),
      api_configs: &recorded,
    };
    let pending = |configs: &HashMap<PageId, ApiConfig>, settings: &Settings, live: &Live| {
      let current = Current {
        pages: std::slice::from_ref(&api),
        redirects: &[],
        drafts: &HashMap::new(),
        content: &HashMap::new(),
        links: &[],
        api_configs: configs,
        settings,
      };
      pending_changes(&current, Some(live))
    };

    // As published: nothing to do.
    let same = pending(&HashMap::from([(api.id, published)]), &site, &live);
    assert!(same.changes.is_empty() && !same.settings);

    // The API page's settings and the site's were saved since.
    let changed = pending(&HashMap::from([(api.id, saved.clone())]), &renamed, &live);
    assert_eq!(changed.changes.iter().map(|c| (c.page_id, c.kinds.clone())).collect::<Vec<_>>(), [(api.id, vec![ChangeKind::Edited])]);
    assert!(changed.settings);

    // A release from before settings were recorded has nothing to compare with.
    let old = Live {
      settings: None,
      api_configs: &HashMap::new(),
      ..live
    };
    let unknown = pending(&HashMap::from([(api.id, saved)]), &renamed, &old);
    assert!(unknown.changes.is_empty() && !unknown.settings);
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
      content: &HashMap::new(),
      settings: None,
      api_configs: &HashMap::new(),
    };
    let now = [guide.clone(), moved.clone()];

    // Moved without a redirect: the old URL is lost and the change says so.
    let without = pending_for(&now, &[], &HashMap::new(), &[], &live);
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
    let with = pending_for(&now, &[redirect("/guide/install/", &moved)], &HashMap::new(), &[], &live);
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
      content: &HashMap::new(),
      settings: None,
      api_configs: &HashMap::new(),
    };

    // The same revision as live: only the missing page shows.
    let unchanged = pending_for(std::slice::from_ref(&kept), &[], &HashMap::from([(kept.id, r1)]), &[], &live);
    assert_eq!(unchanged.changes.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["gone"]);

    let drafts = HashMap::from([(kept.id, r2)]);
    let result = pending_for(&[kept.clone(), fresh.clone()], &[], &drafts, &[], &live);
    // A link from a page that stays to the deleted one is reported by title.
    let broken = pending_for(std::slice::from_ref(&kept), &[], &drafts, &[(kept.id, gone.id)], &live).broken_links;
    assert_eq!((broken[0].source_title.as_str(), broken[0].target_title.as_str()), ("kept", "gone"));

    let kinds: Vec<_> = result.changes.iter().map(|c| (c.title.as_str(), c.kinds.clone())).collect();
    assert_eq!(kinds, [("kept", vec![ChangeKind::Edited]), ("fresh", vec![ChangeKind::New]), ("gone", vec![ChangeKind::Deleted])]);
  }

  #[test]
  fn a_changed_content_hash_is_an_edit_unless_the_release_has_none() {
    let pages: Vec<Page> = ["a", "b", "c"].into_iter().map(|slug| page(slug, None)).collect();
    let revisions: HashMap<_, _> = pages.iter().map(|p| (p.id, RevisionId::new())).collect();
    // `c` was published before content hashes were recorded.
    let recorded = HashMap::from([(pages[0].id, "one".to_string()), (pages[1].id, "two".to_string())]);
    let live = Live {
      pages: &pages,
      redirects: &[],
      revisions: &revisions,
      content: &recorded,
      settings: None,
      api_configs: &HashMap::new(),
    };

    // Same revisions; `a` and `c` now have other content (a snippet they use changed), `b` the same.
    let now = HashMap::from([(pages[0].id, "one, edited".to_string()), (pages[1].id, "two".to_string()), (pages[2].id, "three".to_string())]);
    let current = Current {
      pages: &pages,
      redirects: &[],
      drafts: &revisions,
      content: &now,
      links: &[],
      api_configs: &HashMap::new(),
      settings: &Settings::default(),
    };

    let edited: Vec<_> = pending_changes(&current, Some(&live)).changes.into_iter().map(|c| (c.title, c.kinds)).collect();
    assert_eq!(edited, [("a".to_string(), vec![ChangeKind::Edited])]);
  }
}
