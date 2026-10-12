use yourdocs_core::{Live, Pending, pending_changes};
use yourdocs_store::{DatabaseConnection, all_links, draft_revisions, list_pages, list_redirects, live_release};

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
