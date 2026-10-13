use std::collections::HashMap;

use jiff::Timestamp;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, sea_query::OnConflict};
use yourdocs_core::{Document, EntryKind, Library, PageId, UserId, entry_refs, snippet_cycle};

use crate::{
  StoreError,
  entity::{entry_usage, library_entry},
};

/// One entry of the library as the Library screen lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
  pub id: uuid::Uuid,
  pub kind: EntryKind,
  pub key: String,
  pub text_value: Option<String>,
  pub doc: Option<Document>,
  pub updated_at: String,
  /// The pages that use it directly.
  pub used_in: Vec<PageId>,
}

/// A name usable in a document: lowercase letters, digits, dashes and underscores.
fn check_key(key: &str) -> Result<(), StoreError> {
  let ok = !key.is_empty() && key.len() <= 64 && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'));

  if ok {
    Ok(())
  } else {
    Err(StoreError::Invalid(format!("\"{key}\" is not a usable name: use lowercase letters, digits, dashes and underscores")))
  }
}

pub async fn list_entries(db: &DatabaseConnection) -> Result<Vec<Entry>, StoreError> {
  let rows = library_entry::Entity::find().all(db).await?;
  let usages = entry_usage::Entity::find().all(db).await?;

  let mut entries: Vec<Entry> = rows
    .into_iter()
    .filter_map(|row| {
      let kind = EntryKind::parse(&row.kind)?;
      Some(Entry {
        id: row.id,
        kind,
        key: row.key,
        text_value: row.text_value,
        doc: row.doc.and_then(|d| serde_json::from_str(&d).ok()),
        updated_at: row.updated_at,
        used_in: usages.iter().filter(|u| u.entry_id == row.id).map(|u| u.page_id.into()).collect(),
      })
    })
    .collect();
  entries.sort_by(|a, b| (a.kind.as_str(), &a.key).cmp(&(b.kind.as_str(), &b.key)));

  Ok(entries)
}

/// The variables and snippets, ready for `expand`.
pub async fn load_library(db: &DatabaseConnection) -> Result<Library, StoreError> {
  let mut library = Library::default();

  for entry in list_entries(db).await? {
    match entry.kind {
      EntryKind::Variable => {
        library.variables.insert(entry.key, entry.text_value.unwrap_or_default());
      }
      EntryKind::Snippet => {
        library.snippets.insert(entry.key, entry.doc.unwrap_or_else(Document::empty));
      }
      EntryKind::Term => {}
    }
  }

  Ok(library)
}

async fn upsert(db: &DatabaseConnection, user: UserId, kind: EntryKind, key: &str, text_value: Option<String>, doc: Option<&Document>) -> Result<(), StoreError> {
  library_entry::Entity::insert(library_entry::ActiveModel {
    id: Set(uuid::Uuid::now_v7()),
    kind: Set(kind.as_str().into()),
    key: Set(key.to_string()),
    aliases: Set("[]".into()),
    text_value: Set(text_value),
    doc: Set(doc.map(serde_json::to_string).transpose().map_err(|e| StoreError::Other(e.into()))?),
    link_page_id: Set(None),
    updated_by: Set(user.as_uuid()),
    updated_at: Set(Timestamp::now().to_string()),
  })
  .on_conflict(
    OnConflict::columns([library_entry::Column::Kind, library_entry::Column::Key])
      .update_columns([
        library_entry::Column::TextValue,
        library_entry::Column::Doc,
        library_entry::Column::UpdatedBy,
        library_entry::Column::UpdatedAt,
      ])
      .to_owned(),
  )
  .exec(db)
  .await?;

  Ok(())
}

/// Creates or changes a variable.
pub async fn save_variable(db: &DatabaseConnection, user: UserId, key: &str, value: &str) -> Result<(), StoreError> {
  check_key(key)?;

  upsert(db, user, EntryKind::Variable, key, Some(value.to_string()), None).await
}

/// Creates or changes a snippet. A snippet that would include itself, directly or through other snippets, is refused.
pub async fn save_snippet(db: &DatabaseConnection, user: UserId, key: &str, doc: &Document) -> Result<(), StoreError> {
  check_key(key)?;

  let snippets = load_library(db).await?.snippets;
  if let Some(chain) = snippet_cycle(key, doc, &snippets) {
    return Err(StoreError::Invalid(format!("the snippet {key} would include itself: {}", chain.join(" → "))));
  }

  upsert(db, user, EntryKind::Snippet, key, None, Some(doc)).await
}

/// Deletes an entry, unless pages still use it: the error names them.
pub async fn delete_entry(db: &DatabaseConnection, id: uuid::Uuid) -> Result<(), StoreError> {
  let entries = list_entries(db).await?;
  let entry = entries.iter().find(|e| e.id == id).ok_or(StoreError::NotFound)?;

  // Snippets that include this one count as users too.
  let included_by: Vec<&str> = entries
    .iter()
    .filter(|e| e.kind == EntryKind::Snippet && e.doc.as_ref().is_some_and(|d| entry_refs(d).1.contains(&entry.key)))
    .map(|e| e.key.as_str())
    .collect();

  if entry.kind != EntryKind::Term && (!entry.used_in.is_empty() || !included_by.is_empty()) {
    return Err(StoreError::Conflict(format!(
      "{} {} is still used by {} page(s) and {} snippet(s)",
      entry.kind.as_str(),
      entry.key,
      entry.used_in.len(),
      included_by.len()
    )));
  }

  library_entry::Entity::delete_by_id(id).exec(db).await?;

  Ok(())
}

/// Replaces which entries a page uses with what its document uses now.
pub(crate) async fn replace_usages(db: &impl ConnectionTrait, page: PageId, doc: &Document) -> Result<(), StoreError> {
  entry_usage::Entity::delete_many().filter(entry_usage::Column::PageId.eq(page.as_uuid())).exec(db).await?;

  let (variables, snippets) = entry_refs(doc);
  if variables.is_empty() && snippets.is_empty() {
    return Ok(());
  }

  let rows = library_entry::Entity::find().all(db).await?;
  let used: HashMap<(String, String), uuid::Uuid> = rows.into_iter().map(|r| ((r.kind, r.key), r.id)).collect();
  let usages: Vec<_> = variables
    .iter()
    .filter_map(|k| used.get(&("variable".to_string(), k.clone())))
    .chain(snippets.iter().filter_map(|k| used.get(&("snippet".to_string(), k.clone()))))
    .map(|entry| entry_usage::ActiveModel {
      entry_id: Set(*entry),
      page_id: Set(page.as_uuid()),
    })
    .collect();

  if !usages.is_empty() {
    entry_usage::Entity::insert_many(usages).on_conflict_do_nothing().exec_without_returning(db).await?;
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::connect::test_db;

  fn snippet_doc(include: Option<&str>) -> Document {
    let content = include
      .map(|k| json!([{ "type": "snippet", "attrs": { "key": k } }]))
      .unwrap_or_else(|| json!([{ "type": "paragraph" }]));
    serde_json::from_value(json!({ "type": "doc", "content": content })).unwrap()
  }

  #[tokio::test]
  async fn saves_entries_and_refuses_a_snippet_that_includes_itself() {
    let db = test_db().await;
    let user = UserId::new();

    save_variable(&db, user, "version", "1.0").await.unwrap();
    save_variable(&db, user, "version", "2.0").await.unwrap();
    assert_eq!(load_library(&db).await.unwrap().variables["version"], "2.0");
    assert!(matches!(save_variable(&db, user, "Bad Name", "x").await, Err(StoreError::Invalid(_))));

    save_snippet(&db, user, "prereqs", &snippet_doc(None)).await.unwrap();
    save_snippet(&db, user, "wrapper", &snippet_doc(Some("prereqs"))).await.unwrap();

    // Directly, and through the wrapper.
    for (key, include) in [("prereqs", "prereqs"), ("prereqs", "wrapper")] {
      let err = save_snippet(&db, user, key, &snippet_doc(Some(include))).await.unwrap_err();
      assert!(matches!(&err, StoreError::Invalid(m) if m.contains("include itself")), "{err}");
    }

    // A snippet that others include cannot be deleted.
    let entries = list_entries(&db).await.unwrap();
    let prereqs = entries.iter().find(|e| e.key == "prereqs").unwrap();
    assert!(matches!(delete_entry(&db, prereqs.id).await, Err(StoreError::Conflict(_))));
  }
}
