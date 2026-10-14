use std::collections::{HashMap, HashSet};

use cairn_core::{Current, Document, Live, Operation, PageId, PageKind, Pending, Problem, ProblemKind, doc_problems, expand, page_urls, pending_changes, term_refs, url_problems};
use cairn_render::Pool;
use cairn_store::{DatabaseConnection, Staging, staging};

use crate::{PublishError, content::content_hash};

/// What publishing now would change on the live site, the site settings included, and which live URLs it would break.
pub async fn pending(db: &DatabaseConnection) -> Result<Pending, PublishError> {
  Ok(pending_in(&staging(db).await?))
}

pub(crate) fn pending_in(staging: &Staging) -> Pending {
  let Staging {
    pages,
    redirects,
    revisions: drafts,
    docs,
    links,
    api_configs,
    settings,
    library,
    live,
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

  let state = live.as_ref().map(|l| Live {
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
/// merged); a publish has the specs of its own build and uses `check_with`.
pub async fn check(db: &DatabaseConnection, pool: &Pool, fresh: &OperationAnchors) -> Result<Vec<Problem>, PublishError> {
  let staging = staging(db).await?;
  let mut anchors = OperationAnchors::new();
  for spec in &staging.live_specs {
    anchors.insert(spec.page_id, spec.operations.iter().map(Operation::anchor).collect());
  }
  for (page, known) in fresh {
    anchors.entry(*page).or_default().extend(known.iter().cloned());
  }

  check_with(&staging, pool, &anchors).await
}

/// The anchors of the operations of each API page: what a link to one of them may point at.
pub type OperationAnchors = HashMap<PageId, HashSet<String>>;

pub(crate) async fn check_with(staging: &Staging, pool: &Pool, operations: &OperationAnchors) -> Result<Vec<Problem>, PublishError> {
  let Staging { pages: all, docs, library, .. } = staging;
  let pages: Vec<_> = all.iter().filter(|p| p.kind == PageKind::Page).cloned().collect();
  let empty = Document::empty();
  let mut found = url_problems(&all.iter().filter(|p| p.kind.has_url()).cloned().collect::<Vec<_>>());

  for page in &pages {
    // The page as readers will see it, with variables and snippets in place.
    let (doc, missing) = expand(docs.get(&page.id).unwrap_or(&empty), library);
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
    found.extend(doc_problems(page, doc, all, operations));

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
