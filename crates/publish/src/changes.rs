use yourdocs_core::{Document, Live, PageKind, Pending, Problem, ProblemKind, doc_problems, expand, pending_changes, term_refs, url_problems};
use yourdocs_render::Pool;
use yourdocs_store::{DatabaseConnection, all_links, draft_docs, draft_revisions, list_pages, list_redirects, live_release, load_library};

use crate::PublishError;

/// What publishing now would change on the live site, and which live URLs it would break.
pub async fn pending(db: &DatabaseConnection) -> Result<Pending, PublishError> {
  let pages = list_pages(db).await?;
  let redirects = list_redirects(db).await?;
  let drafts = draft_revisions(db).await?;
  let live = live_release(db).await?;
  let links: Vec<_> = all_links(db).await?.into_iter().map(|l| (l.source, l.target)).collect();

  let state = live.as_ref().map(|l| Live {
    pages: &l.pages,
    redirects: &l.redirects,
    revisions: &l.revisions,
  });

  Ok(pending_changes(&pages, &redirects, &drafts, &links, state.as_ref()))
}

/// Everything that would stop a publish now: links to deleted pages, images without alt text, pages sharing a URL, and blocks or component
/// settings the renderer cannot take. Each names its page.
pub async fn check(db: &DatabaseConnection, pool: &Pool) -> Result<Vec<Problem>, PublishError> {
  let pages: Vec<_> = list_pages(db).await?.into_iter().filter(|p| p.kind == PageKind::Page).collect();
  let docs = draft_docs(db).await?;
  let empty = Document::empty();
  let library = load_library(db).await?;
  let mut found = url_problems(&pages);

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
    found.extend(doc_problems(page, doc, &pages));

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
