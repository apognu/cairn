use fractional_index::FractionalIndex;
use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set};
use yourdocs_core::{OldUrls, Page, PageId, PageKind, Placement, RedirectOrigin, UserId, page_urls, slugify};

use crate::{
  StoreError,
  entity::page::{self, Column},
  redirects::{forget_redirects_from, redirect_to_page},
};

/// All live pages, ordered by parent then position.
pub async fn list_pages(db: &DatabaseConnection) -> Result<Vec<Page>, StoreError> {
  let rows = page::Entity::find().filter(Column::DeletedAt.is_null()).order_by_asc(Column::Position).all(db).await?;

  rows.into_iter().map(into_page).collect()
}

pub async fn create_page(db: &DatabaseConnection, user: UserId, parent: Option<PageId>, title: &str, placement: Placement) -> Result<Page, StoreError> {
  insert(db, user, parent, title, None, placement).await
}

/// Adds an external link to the navigation, which behaves like a page that has no content.
pub async fn create_link(db: &DatabaseConnection, user: UserId, parent: Option<PageId>, title: &str, url: &str, placement: Placement) -> Result<Page, StoreError> {
  let url = check_url(url)?;

  insert(db, user, parent, title, Some(url), placement).await
}

async fn insert(db: &DatabaseConnection, user: UserId, parent: Option<PageId>, title: &str, url: Option<String>, placement: Placement) -> Result<Page, StoreError> {
  let pages = list_pages(db).await?;
  check_parent(&pages, parent)?;

  let slug = unique_slug(&pages, parent, title, None)?;
  let position = position_for(&pages, parent, None, placement)?;
  let now = Timestamp::now().to_string();

  let row = page::ActiveModel {
    id: Set(PageId::new().as_uuid()),
    kind: Set(if url.is_some() { PageKind::Link } else { PageKind::Page }.as_str().into()),
    parent_id: Set(parent.map(|p| p.as_uuid())),
    position: Set(position),
    slug: Set(slug),
    title: Set(title.to_string()),
    external_url: Set(url),
    created_by: Set(user.as_uuid()),
    created_at: Set(now.clone()),
    updated_at: Set(now),
    deleted_at: Set(None),
  }
  .insert(db)
  .await?;

  into_page(row)
}

/// Changes the title and re-derives the slug from it. The URLs of the page and its subtree change with it.
pub async fn rename_page(db: &DatabaseConnection, id: PageId, title: &str, old: OldUrls) -> Result<Page, StoreError> {
  let pages = list_pages(db).await?;
  let current = find(&pages, id)?;
  let slug = unique_slug(&pages, current.parent_id, title, Some(id))?;

  let row = page::ActiveModel {
    id: Set(id.as_uuid()),
    title: Set(title.to_string()),
    slug: Set(slug),
    updated_at: Set(Timestamp::now().to_string()),
    ..Default::default()
  }
  .update(db)
  .await?;

  settle_urls(db, &pages, old, RedirectOrigin::Rename).await?;

  into_page(row)
}

/// Changes where a link points, and its title.
pub async fn edit_link(db: &DatabaseConnection, id: PageId, title: &str, url: &str) -> Result<Page, StoreError> {
  let pages = list_pages(db).await?;
  let current = find(&pages, id)?;

  if current.kind != PageKind::Link {
    return Err(StoreError::Invalid("only links have a URL".into()));
  }

  let url = check_url(url)?;
  let slug = unique_slug(&pages, current.parent_id, title, Some(id))?;

  let row = page::ActiveModel {
    id: Set(id.as_uuid()),
    title: Set(title.to_string()),
    slug: Set(slug),
    external_url: Set(Some(url)),
    updated_at: Set(Timestamp::now().to_string()),
    ..Default::default()
  }
  .update(db)
  .await?;

  into_page(row)
}

/// Moves a page, with its subtree, under `parent` (`None` for the root level).
pub async fn move_page(db: &DatabaseConnection, id: PageId, parent: Option<PageId>, placement: Placement, old: OldUrls) -> Result<Page, StoreError> {
  let pages = list_pages(db).await?;
  let current = find(&pages, id)?;
  check_parent(&pages, parent)?;

  if let Some(parent) = parent
    && (parent == id || ancestors(&pages, parent).contains(&id))
  {
    return Err(StoreError::Invalid("a page cannot move into its own subtree".into()));
  }

  let slug = unique_slug(&pages, parent, &current.title, Some(id))?;
  let position = position_for(&pages, parent, Some(id), placement)?;

  let row = page::ActiveModel {
    id: Set(id.as_uuid()),
    parent_id: Set(parent.map(|p| p.as_uuid())),
    position: Set(position),
    slug: Set(slug),
    updated_at: Set(Timestamp::now().to_string()),
    ..Default::default()
  }
  .update(db)
  .await?;

  settle_urls(db, &pages, old, RedirectOrigin::Move).await?;

  into_page(row)
}

/// Soft-deletes a page and all its descendants; returns the IDs removed.
pub async fn delete_page(db: &DatabaseConnection, id: PageId) -> Result<Vec<PageId>, StoreError> {
  let pages = list_pages(db).await?;
  find(&pages, id)?;

  let doomed: Vec<PageId> = pages.iter().filter(|p| p.id == id || ancestors(&pages, p.id).contains(&id)).map(|p| p.id).collect();
  let now = Timestamp::now().to_string();

  page::Entity::update_many()
    .col_expr(Column::DeletedAt, now.into())
    .filter(Column::Id.is_in(doomed.iter().map(|p| p.as_uuid())))
    .exec(db)
    .await?;

  Ok(doomed)
}

/// After a move or rename: pages whose URL changed get a redirect from the old one (unless `old` says to forget it),
/// and redirects that start at a path a page now occupies are dropped. `before` is the tree as it was.
async fn settle_urls(db: &DatabaseConnection, before: &[Page], old: OldUrls, origin: RedirectOrigin) -> Result<(), StoreError> {
  let after = list_pages(db).await?;
  let (urls_before, urls_after) = (page_urls(before), page_urls(&after));

  if old == OldUrls::Redirect {
    for (id, from) in &urls_before {
      if let Some(to) = urls_after.get(id)
        && to != from
      {
        redirect_to_page(db, from, *id, origin).await?;
      }
    }
  }

  forget_redirects_from(db, urls_after.into_values().collect()).await
}

fn find(pages: &[Page], id: PageId) -> Result<&Page, StoreError> {
  pages.iter().find(|p| p.id == id).ok_or(StoreError::NotFound)
}

fn check_parent(pages: &[Page], parent: Option<PageId>) -> Result<(), StoreError> {
  let Some(parent) = parent else { return Ok(()) };

  match find(pages, parent)?.kind {
    PageKind::Page => Ok(()),
    PageKind::Link => Err(StoreError::Invalid("a link cannot have pages under it".into())),
  }
}

/// Links go to absolute http(s) URLs; anything else (`javascript:`, relative paths) is refused.
fn check_url(url: &str) -> Result<String, StoreError> {
  let url = url.trim();
  let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"));

  match rest {
    Some(rest) if !rest.is_empty() && !rest.starts_with('/') && !url.contains(char::is_whitespace) && url.len() <= 2048 => Ok(url.to_string()),
    _ => Err(StoreError::Invalid("a link needs an absolute http or https URL".into())),
  }
}

/// IDs of every ancestor of `id`, nearest first.
fn ancestors(pages: &[Page], id: PageId) -> Vec<PageId> {
  let mut found = Vec::new();
  let mut current = pages.iter().find(|p| p.id == id).and_then(|p| p.parent_id);

  while let Some(parent) = current {
    // A cycle would loop forever; the checks in `move_page` keep the tree acyclic, this only guards corrupt data.
    if found.contains(&parent) {
      break;
    }

    found.push(parent);
    current = pages.iter().find(|p| p.id == parent).and_then(|p| p.parent_id);
  }

  found
}

fn unique_slug(pages: &[Page], parent: Option<PageId>, title: &str, ignoring: Option<PageId>) -> Result<String, StoreError> {
  let slug = slugify(title);

  if slug.is_empty() {
    return Err(StoreError::Invalid("the title needs at least one letter or digit".into()));
  }

  if pages.iter().any(|p| p.parent_id == parent && p.slug == slug && Some(p.id) != ignoring) {
    return Err(StoreError::Conflict(format!("a sibling already uses the URL segment \"{slug}\"")));
  }

  Ok(slug)
}

/// The fractional position for a page placed among the children of `parent`, ignoring `moving` (the page being moved).
fn position_for(pages: &[Page], parent: Option<PageId>, moving: Option<PageId>, placement: Placement) -> Result<String, StoreError> {
  let siblings: Vec<&Page> = pages.iter().filter(|p| p.parent_id == parent && Some(p.id) != moving).collect();

  let (lower, upper) = match placement {
    Placement::Last => (siblings.last().copied(), None),
    Placement::Before(target) => {
      let index = siblings.iter().position(|p| p.id == target).ok_or(StoreError::NotFound)?;
      (index.checked_sub(1).map(|i| siblings[i]), Some(siblings[index]))
    }
  };

  let decode = |p: &Page| FractionalIndex::from_string(&p.position).map_err(|e| StoreError::Invalid(format!("bad position: {e}")));
  let (lower, upper) = (lower.map(decode).transpose()?, upper.map(decode).transpose()?);

  let index = FractionalIndex::new(lower.as_ref(), upper.as_ref()).ok_or_else(|| StoreError::Invalid("sibling positions are out of order".into()))?;

  Ok(index.to_string())
}

fn into_page(row: page::Model) -> Result<Page, StoreError> {
  let parse = |s: &str| s.parse::<Timestamp>().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")));

  let kind = match row.kind.as_str() {
    "link" => PageKind::Link,
    _ => PageKind::Page,
  };

  Ok(Page {
    id: row.id.into(),
    kind,
    parent_id: row.parent_id.map(Into::into),
    position: row.position,
    slug: row.slug,
    title: row.title,
    external_url: row.external_url,
    created_by: row.created_by.into(),
    created_at: parse(&row.created_at)?,
    updated_at: parse(&row.updated_at)?,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::connect::test_db;

  async fn titles(db: &DatabaseConnection, parent: Option<PageId>) -> Vec<String> {
    list_pages(db).await.unwrap().into_iter().filter(|p| p.parent_id == parent).map(|p| p.title).collect()
  }

  #[tokio::test]
  async fn inserts_between_siblings_and_moves_across_parents() {
    let db = test_db().await;
    let user = UserId::new();

    let a = create_page(&db, user, None, "A", Placement::Last).await.unwrap();
    let c = create_page(&db, user, None, "C", Placement::Last).await.unwrap();
    create_page(&db, user, None, "B", Placement::Before(c.id)).await.unwrap();
    create_page(&db, user, None, "Z", Placement::Before(a.id)).await.unwrap();
    assert_eq!(titles(&db, None).await, ["Z", "A", "B", "C"]);

    // Moving under another parent and back keeps order and does not disturb the old siblings.
    let moved = move_page(&db, c.id, Some(a.id), Placement::Last, OldUrls::Redirect).await.unwrap();
    assert_eq!(moved.parent_id, Some(a.id));
    assert_eq!(titles(&db, None).await, ["Z", "A", "B"]);
    assert_eq!(titles(&db, Some(a.id)).await, ["C"]);

    move_page(&db, c.id, None, Placement::Before(a.id), OldUrls::Redirect).await.unwrap();
    assert_eq!(titles(&db, None).await, ["Z", "C", "A", "B"]);
  }

  #[tokio::test]
  async fn refuses_to_move_a_page_into_its_own_subtree() {
    let db = test_db().await;
    let user = UserId::new();

    let a = create_page(&db, user, None, "A", Placement::Last).await.unwrap();
    let b = create_page(&db, user, Some(a.id), "B", Placement::Last).await.unwrap();
    let c = create_page(&db, user, Some(b.id), "C", Placement::Last).await.unwrap();

    for target in [a.id, b.id, c.id] {
      let err = move_page(&db, a.id, Some(target), Placement::Last, OldUrls::Redirect).await.unwrap_err();
      assert!(matches!(err, StoreError::Invalid(_)), "{err}");
    }
  }

  #[tokio::test]
  async fn rename_rejects_sibling_slug_clashes_and_delete_takes_the_subtree() {
    let db = test_db().await;
    let user = UserId::new();

    let a = create_page(&db, user, None, "Guide", Placement::Last).await.unwrap();
    let b = create_page(&db, user, None, "Other", Placement::Last).await.unwrap();
    create_page(&db, user, Some(b.id), "Child", Placement::Last).await.unwrap();

    assert!(matches!(rename_page(&db, b.id, "guide", OldUrls::Redirect).await, Err(StoreError::Conflict(_))));
    assert_eq!(rename_page(&db, a.id, "The Guide", OldUrls::Redirect).await.unwrap().slug, "the-guide");

    assert_eq!(delete_page(&db, b.id).await.unwrap().len(), 2);
    assert_eq!(titles(&db, None).await, ["The Guide"]);
  }

  #[tokio::test]
  async fn links_hold_a_url_and_no_children() {
    let db = test_db().await;
    let user = UserId::new();

    let section = create_page(&db, user, None, "Integrations", Placement::Last).await.unwrap();
    let link = create_link(&db, user, Some(section.id), "Status page", "https://status.example.com", Placement::Last).await.unwrap();
    assert_eq!(link.kind, PageKind::Link);
    assert_eq!(link.external_url.as_deref(), Some("https://status.example.com"));

    for bad in ["javascript:alert(1)", "/relative", "https://", "https://a b.c", "ftp://x.y"] {
      let err = create_link(&db, user, None, "Bad", bad, Placement::Last).await.unwrap_err();
      assert!(matches!(err, StoreError::Invalid(_)), "{bad}");
    }

    let err = create_page(&db, user, Some(link.id), "Child", Placement::Last).await.unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)));

    let edited = edit_link(&db, link.id, "Status", "http://status.example.com/").await.unwrap();
    assert_eq!((edited.title.as_str(), edited.external_url.as_deref()), ("Status", Some("http://status.example.com/")));
    assert!(matches!(edit_link(&db, section.id, "X", "https://x.y").await, Err(StoreError::Invalid(_))));
  }

  #[tokio::test]
  async fn moves_and_renames_leave_redirects_unless_told_not_to() {
    use crate::redirects::list_redirects;

    let db = test_db().await;
    let user = UserId::new();

    let guide = create_page(&db, user, None, "Guide", Placement::Last).await.unwrap();
    let install = create_page(&db, user, Some(guide.id), "Install", Placement::Last).await.unwrap();
    let other = create_page(&db, user, None, "Other", Placement::Last).await.unwrap();
    let froms = |redirects: Vec<yourdocs_core::Redirect>| redirects.into_iter().map(|r| r.from_path).collect::<Vec<_>>();

    // A move redirects the page and everything under it.
    move_page(&db, guide.id, Some(other.id), Placement::Last, OldUrls::Redirect).await.unwrap();
    assert_eq!(froms(list_redirects(&db).await.unwrap()), ["/guide/", "/guide/install/"]);

    // Moving back gives the paths to the pages again, so the redirects from them go.
    move_page(&db, guide.id, None, Placement::Last, OldUrls::Redirect).await.unwrap();
    assert_eq!(froms(list_redirects(&db).await.unwrap()), ["/other/guide/", "/other/guide/install/"]);

    // Without a redirect (Alt-drop) nothing is recorded.
    move_page(&db, install.id, None, Placement::Last, OldUrls::Forget).await.unwrap();
    assert_eq!(list_redirects(&db).await.unwrap().len(), 2);

    // A rename that keeps the slug changes no URL.
    rename_page(&db, other.id, "OTHER", OldUrls::Redirect).await.unwrap();
    assert_eq!(list_redirects(&db).await.unwrap().len(), 2);

    rename_page(&db, other.id, "Elsewhere", OldUrls::Redirect).await.unwrap();
    let after = froms(list_redirects(&db).await.unwrap());
    assert!(after.contains(&"/other/".to_string()) && after.contains(&"/other/guide/".to_string()), "{after:?}");
  }
}
