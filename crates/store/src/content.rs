use jiff::Timestamp;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use yourdocs_core::{ContentStore, CoreError, Document, Draft, PageId, Revision, RevisionId, UserId, extract_links};

use crate::{
  StoreError,
  entity::{draft, page, revision},
  library::replace_usages,
  links::replace_links,
};

/// `ContentStore` on the SQL database.
///
/// Every save writes an immutable revision, and the draft always points at the newest one, so "latest revision of a page" is the draft's
/// `base_revision_id`. A save is accepted only when it names that revision, which is how two editors on one page are kept from silently
/// overwriting each other.
#[derive(Clone)]
pub struct SqlContentStore {
  db: DatabaseConnection,
}

impl SqlContentStore {
  pub fn new(db: DatabaseConnection) -> Self {
    Self { db }
  }
}

impl From<StoreError> for CoreError {
  fn from(error: StoreError) -> Self {
    match error {
      StoreError::NotFound => CoreError::NotFound,
      StoreError::Invalid(reason) | StoreError::Conflict(reason) => CoreError::Invalid(reason),
      other => CoreError::Other(other.into()),
    }
  }
}

fn other(error: impl Into<anyhow::Error>) -> CoreError {
  CoreError::Other(error.into())
}

impl ContentStore for SqlContentStore {
  async fn draft(&self, page: PageId) -> Result<Option<Draft>, CoreError> {
    let row = draft::Entity::find_by_id(page.as_uuid()).one(&self.db).await.map_err(other)?;

    row.map(into_draft).transpose()
  }

  async fn save_draft(&self, page: PageId, doc: Document, base: Option<RevisionId>, user: UserId) -> Result<Draft, CoreError> {
    let page_row = page::Entity::find_by_id(page.as_uuid()).filter(page::Column::DeletedAt.is_null()).one(&self.db).await.map_err(other)?;
    let Some(page_row) = page_row else { return Err(CoreError::NotFound) };

    let now = Timestamp::now().to_string();
    let json = serde_json::to_string(&doc).map_err(other)?;
    let revision_id = RevisionId::new();

    // The revision and the draft move together: if the draft is stale, the revision is rolled back with it.
    let tx = self.db.begin().await.map_err(other)?;

    revision::ActiveModel {
      id: Set(revision_id.as_uuid()),
      page_id: Set(page.as_uuid()),
      doc: Set(json.clone()),
      title: Set(page_row.title),
      slug: Set(page_row.slug),
      author_id: Set(user.as_uuid()),
      created_at: Set(now.clone()),
    }
    .insert(&tx)
    .await
    .map_err(other)?;

    let moved = match base {
      // First save: only succeeds while no draft exists.
      None => {
        let inserted = draft::Entity::insert(draft::ActiveModel {
          page_id: Set(page.as_uuid()),
          doc: Set(json.clone()),
          base_revision_id: Set(Some(revision_id.as_uuid())),
          updated_by: Set(user.as_uuid()),
          updated_at: Set(now.clone()),
        })
        .on_conflict_do_nothing_on([draft::Column::PageId])
        .exec(&tx)
        .await
        .map_err(other)?;

        matches!(inserted, sea_orm::TryInsertResult::Inserted(_))
      }
      Some(base) => {
        let updated = draft::Entity::update_many()
          .set(draft::ActiveModel {
            doc: Set(json.clone()),
            base_revision_id: Set(Some(revision_id.as_uuid())),
            updated_by: Set(user.as_uuid()),
            updated_at: Set(now.clone()),
            ..Default::default()
          })
          .filter(draft::Column::PageId.eq(page.as_uuid()))
          .filter(draft::Column::BaseRevisionId.eq(base.as_uuid()))
          .exec(&tx)
          .await
          .map_err(other)?;

        updated.rows_affected == 1
      }
    };

    if !moved {
      tx.rollback().await.map_err(other)?;
      return Err(CoreError::Conflict { base });
    }

    // Backlinks and delete warnings read these; they move with the draft or not at all.
    replace_links(&tx, page, &extract_links(&doc)).await?;
    replace_usages(&tx, page, &doc).await?;

    tx.commit().await.map_err(other)?;

    Ok(Draft {
      page_id: page,
      doc,
      base_revision_id: Some(revision_id),
      updated_by: user,
      updated_at: now.parse().map_err(other)?,
    })
  }

  async fn revision(&self, id: RevisionId) -> Result<Revision, CoreError> {
    let row = revision::Entity::find_by_id(id.as_uuid()).one(&self.db).await.map_err(other)?;

    into_revision(row.ok_or(CoreError::NotFound)?)
  }

  async fn latest_revision(&self, page: PageId) -> Result<Option<Revision>, CoreError> {
    let row = revision::Entity::find()
      .filter(revision::Column::PageId.eq(page.as_uuid()))
      .order_by_desc(revision::Column::Id)
      .one(&self.db)
      .await
      .map_err(other)?;

    row.map(into_revision).transpose()
  }

  async fn ensure_revision(&self, page: PageId, user: UserId) -> Result<Revision, CoreError> {
    if let Some(latest) = self.latest_revision(page).await? {
      return Ok(latest);
    }

    let page_row = page::Entity::find_by_id(page.as_uuid()).one(&self.db).await.map_err(other)?.ok_or(CoreError::NotFound)?;
    let row = revision::ActiveModel {
      id: Set(RevisionId::new().as_uuid()),
      page_id: Set(page.as_uuid()),
      doc: Set(serde_json::to_string(&Document::empty()).map_err(other)?),
      title: Set(page_row.title),
      slug: Set(page_row.slug),
      author_id: Set(user.as_uuid()),
      created_at: Set(Timestamp::now().to_string()),
    }
    .insert(&self.db)
    .await
    .map_err(other)?;

    into_revision(row)
  }
}

fn into_draft(row: draft::Model) -> Result<Draft, CoreError> {
  Ok(Draft {
    page_id: row.page_id.into(),
    doc: serde_json::from_str(&row.doc).map_err(other)?,
    base_revision_id: row.base_revision_id.map(Into::into),
    updated_by: row.updated_by.into(),
    updated_at: row.updated_at.parse().map_err(other)?,
  })
}

fn into_revision(row: revision::Model) -> Result<Revision, CoreError> {
  Ok(Revision {
    id: row.id.into(),
    page_id: row.page_id.into(),
    doc: serde_json::from_str(&row.doc).map_err(other)?,
    title: row.title,
    author_id: row.author_id.into(),
    created_at: row.created_at.parse().map_err(other)?,
  })
}

#[cfg(test)]
mod tests {
  use yourdocs_core::{Node, Placement};

  use super::*;
  use crate::{connect::test_db, create_page};

  fn doc(text: &str) -> Document {
    Document(Node {
      kind: "doc".into(),
      attrs: None,
      content: Some(vec![Node {
        kind: "paragraph".into(),
        attrs: None,
        content: Some(vec![Node {
          kind: "text".into(),
          attrs: None,
          content: None,
          marks: None,
          text: Some(text.into()),
        }]),
        marks: None,
        text: None,
      }]),
      marks: None,
      text: None,
    })
  }

  // A save built on an old revision must be refused, or one editor's work silently replaces another's.
  #[tokio::test]
  async fn refuses_saves_based_on_an_old_revision() {
    let db = test_db().await;
    let user = UserId::new();
    let page = create_page(&db, user, None, "A", Placement::Last).await.unwrap().id;
    let store = SqlContentStore::new(db);

    assert!(store.draft(page).await.unwrap().is_none());

    let first = store.save_draft(page, doc("one"), None, user).await.unwrap();
    let second = store.save_draft(page, doc("two"), first.base_revision_id, user).await.unwrap();

    // Both of these are built on a state that is gone.
    assert!(matches!(store.save_draft(page, doc("late"), first.base_revision_id, user).await, Err(CoreError::Conflict { .. })));
    assert!(matches!(store.save_draft(page, doc("late"), None, user).await, Err(CoreError::Conflict { .. })));

    let draft = store.draft(page).await.unwrap().unwrap();
    assert_eq!(draft.doc, doc("two"));
    assert_eq!(draft.base_revision_id, second.base_revision_id);
    assert_eq!(store.latest_revision(page).await.unwrap().unwrap().id, second.base_revision_id.unwrap());
    // The refused saves left no revision behind.
    assert_eq!(store.revision(first.base_revision_id.unwrap()).await.unwrap().doc, doc("one"));
  }

  #[tokio::test]
  async fn unknown_page_is_not_found() {
    let store = SqlContentStore::new(test_db().await);

    assert!(matches!(store.save_draft(PageId::new(), doc("x"), None, UserId::new()).await, Err(CoreError::NotFound)));
  }

  #[tokio::test]
  async fn saving_a_draft_records_what_it_links_to() {
    use crate::{all_links, tree::create_page};
    use yourdocs_core::{LinkKind, Placement};

    let db = test_db().await;
    let store = SqlContentStore::new(db.clone());
    let user = UserId::new();
    let a = create_page(&db, user, None, "A", Placement::Last).await.unwrap();
    let b = create_page(&db, user, None, "B", Placement::Last).await.unwrap();

    let with_link = |target: PageId| -> Document {
      serde_json::from_value(serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
        { "type": "text", "text": "see", "marks": [{ "type": "link", "attrs": { "pageId": target.to_string() } }] }
      ] }] }))
      .unwrap()
    };

    let draft = store.save_draft(a.id, with_link(b.id), None, user).await.unwrap();
    let links = all_links(&db).await.unwrap();
    assert_eq!((links.len(), links[0].source, links[0].target, links[0].kind), (1, a.id, b.id, LinkKind::Inline));

    // A new save replaces the old set; a link to nothing known is not recorded.
    store.save_draft(a.id, with_link(PageId::new()), draft.base_revision_id, user).await.unwrap();
    assert!(all_links(&db).await.unwrap().is_empty());
  }
}
