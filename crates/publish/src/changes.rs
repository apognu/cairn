use std::collections::{HashMap, HashSet};

use cairn_core::{
  Current, Document, Live, Operation, Page, PageId, PageKind, Pending, Problem, ProblemKind, Pulled, doc_problems, expand, heading_anchors, hotfix_pages, page_urls, pending_changes, term_refs,
  url_problems,
};
use cairn_render::Pool;
use cairn_store::{DatabaseConnection, LiveRelease, Staging, staging};

use crate::{PublishError, content::content_hash};

/// What publishing now would change on the live site, the site settings included, and which live URLs it would break.
pub async fn pending(db: &DatabaseConnection) -> Result<Pending, PublishError> {
  Ok(pending_in(&staging(db).await?))
}

pub(crate) fn pending_in(staging: &Staging) -> Pending {
  pending_against(staging, staging.live.as_ref())
}

/// What publishing now would change compared with `against`, which is the live release, or a preview to see what happened since it.
pub(crate) fn pending_against(staging: &Staging, against: Option<&LiveRelease>) -> Pending {
  let Staging {
    pages,
    redirects,
    revisions: drafts,
    docs,
    links,
    api_configs,
    settings,
    library,
    ..
  } = staging;
  let links: Vec<_> = links.iter().map(|l| (l.source, l.target)).collect();

  // What each page would be published with, hashed as the publisher records it.
  let urls = page_urls(pages);
  let empty = Document::empty();
  let content: HashMap<PageId, String> = pages
    .iter()
    .filter(|p| p.kind == PageKind::Page)
    .map(|p| (p.id, content_hash(&expand(docs.get(&p.id).unwrap_or(&empty), library).0, pages, &urls, library)))
    .collect();

  let state = against.map(|l| Live {
    pages: &l.pages,
    redirects: &l.redirects,
    revisions: &l.revisions,
    content: &l.content,
    settings: l.settings.as_ref(),
    api_configs: &l.api_configs,
  });
  let current = Current {
    pages,
    redirects,
    drafts,
    content: &content,
    links: &links,
    api_configs,
    settings,
  };

  pending_changes(&current, state.as_ref())
}

/// Everything that would stop a publish now: links to deleted pages, images without alt text, pages sharing a URL, and blocks or component
/// settings the renderer cannot take. Each names its page.
///
/// Links to operations of API pages are compared with what the live release recorded, plus `fresh` (what a check of the settings just
/// merged); a publish has the specs of its own build and uses `check_with`. Links to a section are compared with the drafts' headings.
pub async fn check(db: &DatabaseConnection, pool: &Pool, fresh: &OperationAnchors) -> Result<Vec<Problem>, PublishError> {
  let staging = staging(db).await?;
  let mut anchors = OperationAnchors::new();
  for spec in &staging.live_specs {
    anchors.insert(spec.page_id, spec.operations.iter().map(Operation::anchor).collect());
  }
  for (page, known) in fresh {
    anchors.entry(*page).or_default().extend(known.iter().cloned());
  }

  check_with(&staging, &staging.pages, pool, &anchors).await
}

/// The anchors of the operations of each API page: what a link to one of them may point at.
pub type OperationAnchors = HashMap<PageId, HashSet<String>>;

/// `linkable` are the pages a link may point to without being broken: all pages that exist, even those a hotfix leaves out.
pub(crate) async fn check_with(staging: &Staging, linkable: &[Page], pool: &Pool, operations: &OperationAnchors) -> Result<Vec<Problem>, PublishError> {
  let Staging { pages: all, docs, library, .. } = staging;
  let pages: Vec<_> = all.iter().filter(|p| p.kind == PageKind::Page).cloned().collect();
  let empty = Document::empty();
  let mut found = url_problems(&all.iter().filter(|p| p.kind.has_url()).cloned().collect::<Vec<_>>());

  // The pages as readers will see them, with variables and snippets in place: their headings are what links to a section point at.
  let expanded: Vec<_> = pages.iter().map(|page| (page, expand(docs.get(&page.id).unwrap_or(&empty), library))).collect();
  let mut anchors = operations.clone();
  for (page, (doc, _)) in &expanded {
    anchors.insert(page.id, heading_anchors(doc));
  }

  for (page, (doc, missing)) in expanded {
    let doc = &doc;
    let unknown_terms = term_refs(doc).into_iter().filter(|t| !library.terms.contains_key(t)).map(|t| format!("term {t}"));
    for key in missing.into_iter().chain(unknown_terms) {
      found.push(Problem {
        page_id: page.id,
        page_title: page.title.clone(),
        kind: ProblemKind::InvalidProps,
        message: format!("uses a {key} that does not exist"),
      });
    }
    found.extend(doc_problems(page, doc, linkable, &anchors));

    let value = serde_json::to_value(doc).map_err(anyhow::Error::from)?;
    for message in pool.validate(&value).await.map_err(|source| PublishError::Render { page: page.title.clone(), source })? {
      found.push(Problem {
        page_id: page.id,
        page_title: page.title.clone(),
        kind: ProblemKind::InvalidProps,
        message,
      });
    }
  }

  Ok(found)
}

/// A link from a page in a hotfix to a page the hotfix leaves out: it shows as plain text until that page is published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainLink {
  pub source_title: String,
  pub target_id: PageId,
  pub target_title: String,
}

/// What a hotfix of the picked pages would also take, and which of their links would show as plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixPlan {
  pub pulled: Vec<Pulled>,
  pub plain_links: Vec<PlainLink>,
}

pub async fn hotfix_plan(db: &DatabaseConnection, picked: &HashSet<PageId>) -> Result<HotfixPlan, PublishError> {
  let staging = staging(db).await?;
  let no_pages = Vec::new();
  let live_pages = staging.live.as_ref().map_or(&no_pages, |l| &l.pages);
  let (tree, pulled) = hotfix_pages(live_pages, &staging.pages, picked);
  let included: HashSet<PageId> = picked.iter().copied().chain(pulled.iter().map(|p| p.page_id)).collect();
  let title = |id: PageId| staging.pages.iter().find(|p| p.id == id).map(|p| p.title.clone());

  let mut plain_links: Vec<PlainLink> = staging
    .links
    .iter()
    .filter(|l| included.contains(&l.source) && !tree.iter().any(|p| p.id == l.target))
    .filter_map(|l| {
      Some(PlainLink {
        source_title: title(l.source)?,
        target_id: l.target,
        target_title: title(l.target)?,
      })
    })
    .collect();
  plain_links.sort_by(|a, b| (&a.source_title, &a.target_title).cmp(&(&b.source_title, &b.target_title)));
  plain_links.dedup();

  Ok(HotfixPlan { pulled, plain_links })
}
