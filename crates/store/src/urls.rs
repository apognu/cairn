//! Custom URLs and aliases: the URL decisions an author can override per page.

use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use yourdocs_core::{Inactive, OldUrls, Page, PageId, PageKind, Redirect, RedirectOrigin, page_urls, resolve_redirect};

use crate::{
  StoreError,
  entity::{page, redirect},
  redirects::{list_redirects, redirect_to_page, redirect_to_url},
  tree::{check_url, list_pages, settle_urls},
};

/// The first path segments the published site uses for its own files.
const RESERVED: [&str; 3] = ["assets", "_a", "pagefind"];

/// A path as stored: lowercase, with a leading and a trailing slash, made of letters, digits, `-`, `_` and `.` (never `.` or `..` alone).
pub fn normalize_path(input: &str) -> Result<String, StoreError> {
  let path = input.trim().to_lowercase();
  let invalid = |why: &str| StoreError::Invalid(format!("\"{}\" is not a usable path: {why}", input.trim()));

  let Some(rest) = path.strip_prefix('/') else {
    return Err(invalid("it must start with a slash"));
  };
  let rest = rest.trim_end_matches('/');

  if rest.is_empty() {
    return Err(invalid("the site root belongs to the home page"));
  }
  if rest.len() > 200 {
    return Err(invalid("it is too long"));
  }

  for segment in rest.split('/') {
    let ok = !segment.is_empty() && segment != "." && segment != ".." && segment.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !ok {
      return Err(invalid("use letters, digits, dashes and single slashes"));
    }
  }

  if RESERVED.contains(&rest.split('/').next().unwrap_or_default()) {
    return Err(invalid("the site uses that first segment for its own files"));
  }

  Ok(format!("/{rest}/"))
}

fn page_for_update(pages: &[Page], id: PageId) -> Result<&Page, StoreError> {
  let page = pages.iter().find(|p| p.id == id).ok_or(StoreError::NotFound)?;

  match page.kind {
    PageKind::Page => Ok(page),
    PageKind::Link => Err(StoreError::Invalid("a link has no URL of its own".into())),
  }
}

fn title_of(pages: &[Page], id: PageId) -> String {
  pages.iter().find(|p| p.id == id).map_or_else(|| "a deleted page".to_string(), |p| format!("“{}”", p.title))
}

/// Why `path` cannot lead to `target`: it is already another page's URL or another redirect's start. `pages` is the tree as it would be.
fn check_free(pages: &[Page], redirects: &[Redirect], path: &str, target: &[PageId]) -> Result<(), StoreError> {
  for (id, url) in page_urls(pages) {
    if url == path && !target.contains(&id) {
      return Err(StoreError::Conflict(format!("{path} is already the URL of {}", title_of(pages, id))));
    }
  }

  for r in redirects.iter().filter(|r| r.from_path == path) {
    match (&r.to_page_id, &r.to_url) {
      (Some(id), _) if target.contains(id) => {}
      (Some(id), _) => return Err(StoreError::Conflict(format!("{path} is already an alias of {}", title_of(pages, *id)))),
      (None, Some(url)) => return Err(StoreError::Conflict(format!("{path} already redirects to {url}"))),
      (None, None) => {}
    }
  }

  Ok(())
}

/// Gives the page a fixed URL, or takes it back to the one derived from the tree (`None`). The pages below follow it. The old URLs
/// are kept as redirects unless `old` says to forget them.
pub async fn set_custom_url(db: &DatabaseConnection, id: PageId, path: Option<&str>, old: OldUrls) -> Result<Page, StoreError> {
  let before = list_pages(db).await?;
  page_for_update(&before, id)?;
  let path = path.map(normalize_path).transpose()?;

  // The page and everything below it get new URLs: each must be free.
  let mut after = before.clone();
  if let Some(page) = after.iter_mut().find(|p| p.id == id) {
    page.path_override = path.clone();
  }
  let redirects = list_redirects(db).await?;
  let (urls_before, urls_after) = (page_urls(&before), page_urls(&after));

  for (page_id, url) in &urls_after {
    if urls_before.get(page_id) != Some(url) {
      check_free(&after, &redirects, url, &[*page_id])?;
    }
  }

  let row = page::ActiveModel {
    id: Set(id.as_uuid()),
    path_override: Set(path),
    updated_at: Set(Timestamp::now().to_string()),
    ..Default::default()
  }
  .update(db)
  .await?;

  settle_urls(db, &before, old, RedirectOrigin::Rename).await?;

  list_pages(db).await?.into_iter().find(|p| p.id == Into::<PageId>::into(row.id)).ok_or(StoreError::NotFound)
}

/// Every old URL that redirects to the page.
pub async fn list_aliases(db: &DatabaseConnection, id: PageId) -> Result<Vec<Redirect>, StoreError> {
  Ok(list_redirects(db).await?.into_iter().filter(|r| r.to_page_id == Some(id)).collect())
}

/// Adds a redirect from `path` to the page by hand.
pub async fn add_alias(db: &DatabaseConnection, id: PageId, path: &str) -> Result<(), StoreError> {
  let pages = list_pages(db).await?;
  page_for_update(&pages, id)?;
  let path = normalize_path(path)?;

  if page_urls(&pages).get(&id) == Some(&path) {
    return Err(StoreError::Invalid(format!("{path} is already this page's URL")));
  }
  check_free(&pages, &list_redirects(db).await?, &path, &[id])?;

  redirect_to_page(db, &path, id, RedirectOrigin::Manual).await
}

/// Where a hand-made redirect leads.
pub enum RedirectTarget {
  Page(PageId),
  /// An absolute http(s) URL, or a path on this site.
  Url(String),
}

/// Adds a redirect from `from` that is not tied to a page's own URL changes: to a page or to a URL.
pub async fn add_redirect(db: &DatabaseConnection, from: &str, to: RedirectTarget) -> Result<(), StoreError> {
  let pages = list_pages(db).await?;
  let redirects = list_redirects(db).await?;
  let from = normalize_path(from)?;

  let (to_page_id, to_url) = match to {
    RedirectTarget::Page(id) => (Some(page_for_update(&pages, id)?.id), None),
    RedirectTarget::Url(url) if url.trim().starts_with('/') => (None, Some(normalize_path(&url)?)),
    RedirectTarget::Url(url) => (None, Some(check_url(&url)?)),
  };

  check_free(&pages, &redirects, &from, to_page_id.as_slice())?;

  let candidate = Redirect {
    from_path: from.clone(),
    to_page_id,
    to_url: to_url.clone(),
    origin: RedirectOrigin::Manual,
    created_at: Timestamp::now(),
  };
  let mut all = redirects;
  all.push(candidate.clone());
  if resolve_redirect(&candidate, &all, &pages) == Err(Inactive::Loop) {
    return Err(StoreError::Invalid(format!("{from} would make a redirect loop")));
  }

  match (to_page_id, to_url) {
    (Some(id), _) => redirect_to_page(db, &from, id, RedirectOrigin::Manual).await,
    (None, Some(url)) => redirect_to_url(db, &from, &url).await,
    (None, None) => Ok(()),
  }
}

/// Deletes a redirect, whoever it points to.
pub async fn remove_redirect(db: &DatabaseConnection, path: &str) -> Result<(), StoreError> {
  let result = redirect::Entity::delete_many().filter(redirect::Column::FromPath.eq(path)).exec(db).await?;

  if result.rows_affected == 0 { Err(StoreError::NotFound) } else { Ok(()) }
}

pub async fn remove_alias(db: &DatabaseConnection, id: PageId, path: &str) -> Result<(), StoreError> {
  let result = redirect::Entity::delete_many()
    .filter(redirect::Column::FromPath.eq(path))
    .filter(redirect::Column::ToPageId.eq(id.as_uuid()))
    .exec(db)
    .await?;

  if result.rows_affected == 0 { Err(StoreError::NotFound) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
  use yourdocs_core::{Placement, UserId};

  use super::*;
  use crate::{connect::test_db, tree::create_page};

  #[test]
  fn normalizes_paths_and_refuses_the_rest() {
    for (input, expected) in [
      ("/Webhooks", Some("/webhooks/")),
      ("  /a/b// ", Some("/a/b/")),
      ("/v1.2/x_y", Some("/v1.2/x_y/")),
      ("hooks", None),
      ("/", None),
      ("/a//b", None),
      ("/a/../b", None),
      ("/assets/x", None),
      ("/a b", None),
    ] {
      assert_eq!(normalize_path(input).ok().as_deref(), expected, "{input}");
    }
  }

  #[tokio::test]
  async fn conflicts_name_the_page_that_owns_the_path() {
    let db = test_db().await;
    let user = UserId::new();
    let rollbacks = create_page(&db, user, None, "Rollbacks", Placement::Last).await.unwrap();
    let webhooks = create_page(&db, user, None, "Webhooks", Placement::Last).await.unwrap();

    // Another page's URL, by derivation or by custom URL.
    let err = add_alias(&db, webhooks.id, "/rollbacks").await.unwrap_err();
    assert!(matches!(&err, StoreError::Conflict(m) if m.contains("Rollbacks")), "{err}");
    let err = set_custom_url(&db, webhooks.id, Some("/rollbacks/"), OldUrls::Redirect).await.unwrap_err();
    assert!(matches!(&err, StoreError::Conflict(m) if m.contains("Rollbacks")), "{err}");

    // Another page's alias.
    add_alias(&db, rollbacks.id, "/undo").await.unwrap();
    let err = add_alias(&db, webhooks.id, "/undo/").await.unwrap_err();
    assert!(matches!(&err, StoreError::Conflict(m) if m.contains("alias of “Rollbacks”")), "{err}");

    // Its own URL is no alias, an own alias is fine twice, and removing it frees the path.
    assert!(matches!(add_alias(&db, webhooks.id, "/webhooks").await, Err(StoreError::Invalid(_))));
    add_alias(&db, rollbacks.id, "/undo").await.unwrap();
    remove_alias(&db, rollbacks.id, "/undo/").await.unwrap();
    add_alias(&db, webhooks.id, "/undo").await.unwrap();
  }

  #[tokio::test]
  async fn manual_redirects_refuse_clashes_and_loops() {
    let db = test_db().await;
    let user = UserId::new();
    let docs = create_page(&db, user, None, "Docs", Placement::Last).await.unwrap();

    add_redirect(&db, "/old", RedirectTarget::Page(docs.id)).await.unwrap();
    add_redirect(&db, "/ext", RedirectTarget::Url("https://example.com/x".into())).await.unwrap();

    // Taken by a page, or by a redirect to another place.
    for (from, to) in [("/docs", "https://example.com"), ("/ext", "/docs")] {
      assert!(add_redirect(&db, from, RedirectTarget::Url(to.into())).await.is_err(), "{from}");
    }

    // The second half of a loop is refused.
    add_redirect(&db, "/loop-b", RedirectTarget::Url("/loop-a".into())).await.unwrap();
    let err = add_redirect(&db, "/loop-a", RedirectTarget::Url("/loop-b".into())).await.unwrap_err();
    assert!(matches!(err, StoreError::Invalid(m) if m.contains("loop")));

    remove_redirect(&db, "/ext/").await.unwrap();
    assert!(matches!(remove_redirect(&db, "/ext/").await, Err(StoreError::NotFound)));
  }

  #[tokio::test]
  async fn a_custom_url_keeps_the_old_one_and_follows_the_page() {
    let db = test_db().await;
    let user = UserId::new();
    let guide = create_page(&db, user, None, "Guide", Placement::Last).await.unwrap();
    let hooks = create_page(&db, user, Some(guide.id), "Webhooks", Placement::Last).await.unwrap();
    let retries = create_page(&db, user, Some(hooks.id), "Retries", Placement::Last).await.unwrap();

    let page = set_custom_url(&db, hooks.id, Some("/hooks"), OldUrls::Redirect).await.unwrap();
    assert_eq!(page.path_override.as_deref(), Some("/hooks/"));

    let aliases: Vec<_> = list_aliases(&db, hooks.id).await.unwrap().into_iter().map(|r| r.from_path).collect();
    assert_eq!(aliases, ["/guide/webhooks/"]);
    let below: Vec<_> = list_aliases(&db, retries.id).await.unwrap().into_iter().map(|r| r.from_path).collect();
    assert_eq!(below, ["/guide/webhooks/retries/"]);

    // Back to the derived URL: the custom one becomes an alias, and the paths the pages occupy again stop redirecting.
    set_custom_url(&db, hooks.id, None, OldUrls::Redirect).await.unwrap();
    let aliases: Vec<_> = list_aliases(&db, hooks.id).await.unwrap().into_iter().map(|r| r.from_path).collect();
    assert_eq!(aliases, ["/hooks/"]);
  }
}
