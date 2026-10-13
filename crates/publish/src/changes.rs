use std::collections::{HashMap, HashSet};

use yourdocs_core::{Document, Live, Operation, PageId, PageKind, Pending, Problem, ProblemKind, doc_problems, expand, pending_changes, term_refs, url_problems};
use yourdocs_render::Pool;
use yourdocs_store::{
  DatabaseConnection, all_links, draft_docs, draft_revisions, latest_live_release, list_api_configs, list_pages, list_redirects, live_release, load_library, load_settings, release_specs,
};

use crate::PublishError;

/// What publishing now would change on the live site, the site settings included, and which live URLs it would break.
pub async fn pending(db: &DatabaseConnection) -> Result<Pending, PublishError> {
  let pages = list_pages(db).await?;
  let redirects = list_redirects(db).await?;
  let drafts = draft_revisions(db).await?;
  let live = live_release(db).await?;
  let links: Vec<_> = all_links(db).await?.into_iter().map(|l| (l.source, l.target)).collect();
  let api_configs = list_api_configs(db).await?;
  let settings = load_settings(db).await?;

  let state = live.as_ref().map(|l| Live {
    pages: &l.pages,
    redirects: &l.redirects,
    revisions: &l.revisions,
    settings: l.settings.as_ref(),
    api_configs: &l.api_configs,
  });

  Ok(pending_changes(&pages, &redirects, &drafts, &links, &api_configs, &settings, state.as_ref()))
}

/// Everything that would stop a publish now: links to deleted pages, images without alt text, pages sharing a URL, and blocks or component
/// settings the renderer cannot take. Each names its page.
///
/// Links to operations of API pages are compared with what the live release recorded, plus `fresh` (what a check of the settings just
/// merged); a publish has the specs of its own build and uses `check_with`.
pub async fn check(db: &DatabaseConnection, pool: &Pool, fresh: &OperationAnchors) -> Result<Vec<Problem>, PublishError> {
  let mut anchors = OperationAnchors::new();
  if let Some(live) = latest_live_release(db).await? {
    for spec in release_specs(db, live.id).await? {
      anchors.insert(spec.page_id, spec.operations.iter().map(Operation::anchor).collect());
    }
  }
  for (page, known) in fresh {
    anchors.entry(*page).or_default().extend(known.iter().cloned());
  }

  check_with(db, pool, &anchors).await
}

/// The anchors of the operations of each API page: what a link to one of them may point at.
pub type OperationAnchors = HashMap<PageId, HashSet<String>>;

pub(crate) async fn check_with(db: &DatabaseConnection, pool: &Pool, operations: &OperationAnchors) -> Result<Vec<Problem>, PublishError> {
  let all = list_pages(db).await?;
  let pages: Vec<_> = all.iter().filter(|p| p.kind == PageKind::Page).cloned().collect();
  let docs = draft_docs(db).await?;
  let empty = Document::empty();
  let library = load_library(db).await?;
  let mut found = url_problems(&all.iter().filter(|p| p.kind.has_url()).cloned().collect::<Vec<_>>());

  for page in &pages {
    // The page as readers will see it, with variables and snippets in place.
    let (doc, missing) = expand(docs.get(&page.id).unwrap_or(&empty), &library);
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
    found.extend(doc_problems(page, doc, &all, operations));

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
