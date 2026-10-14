use std::collections::{HashMap, HashSet};

use cairn_core::{Document, EntryKind, Library, PageId, UserId, entry_refs, snippet_cycle, term_refs};
use jiff::Timestamp;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, sea_query::OnConflict};

use crate::{
  StoreError,
  entity::{draft, entry_usage, library_entry},
  tree::list_pages,
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
      EntryKind::Term => {
        library.terms.insert(entry.key, entry.text_value.unwrap_or_default());
      }
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

/// Creates or changes a glossary term: the words as written, and a short definition.
pub async fn save_term(db: &DatabaseConnection, user: UserId, term: &str, definition: &str) -> Result<(), StoreError> {
  let term = term.trim();
  if term.is_empty() || term.chars().count() > 80 {
    return Err(StoreError::Invalid("a term is 1 to 80 characters".into()));
  }
  if definition.trim().is_empty() || definition.chars().count() > 400 {
    return Err(StoreError::Invalid("a definition is 1 to 400 characters".into()));
  }

  upsert(db, user, EntryKind::Term, term, Some(definition.trim().to_string()), None).await
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

/// Deletes an entry, unless live pages or other snippets still use it.
pub async fn delete_entry(db: &DatabaseConnection, id: uuid::Uuid) -> Result<(), StoreError> {
  let entries = list_entries(db).await?;
  let entry = entries.iter().find(|e| e.id == id).ok_or(StoreError::NotFound)?;

  let uses = |doc: &Document| match entry.kind {
    EntryKind::Variable => entry_refs(doc).0.contains(&entry.key),
    EntryKind::Snippet => entry_refs(doc).1.contains(&entry.key),
    EntryKind::Term => term_refs(doc).contains(&entry.key),
  };

  // Read from the documents, not the usage table: that table misses a reference written before the entry existed, and counts pages in
  // the trash, which would make the entry impossible to delete.
  let live: HashSet<uuid::Uuid> = list_pages(db).await?.iter().map(|p| p.id.as_uuid()).collect();
  let pages = draft::Entity::find()
    .all(db)
    .await?
    .into_iter()
    .filter(|d| live.contains(&d.page_id))
    .filter_map(|d| serde_json::from_str::<Document>(&d.doc).ok())
    .filter(|doc| uses(doc))
    .count();
  let snippets = entries.iter().filter(|e| e.kind == EntryKind::Snippet && e.id != entry.id && e.doc.as_ref().is_some_and(uses)).count();

  if pages > 0 || snippets > 0 {
    return Err(StoreError::Conflict(format!(
      "{} {} is still used by {pages} page(s) and {snippets} snippet(s)",
      entry.kind.as_str(),
      entry.key,
    )));
  }

  library_entry::Entity::delete_by_id(id).exec(db).await?;

  Ok(())
}

/// Replaces which entries a page uses with what its document uses now.
pub(crate) async fn replace_usages(db: &impl ConnectionTrait, page: PageId, doc: &Document) -> Result<(), StoreError> {
  entry_usage::Entity::delete_many().filter(entry_usage::Column::PageId.eq(page.as_uuid())).exec(db).await?;

  let (variables, snippets) = entry_refs(doc);
  let terms = term_refs(doc);
  if variables.is_empty() && snippets.is_empty() && terms.is_empty() {
    return Ok(());
  }

  let rows = library_entry::Entity::find().all(db).await?;
  let used: HashMap<(String, String), uuid::Uuid> = rows.into_iter().map(|r| ((r.kind, r.key), r.id)).collect();
  let usages: Vec<_> = variables
    .iter()
    .filter_map(|k| used.get(&("variable".to_string(), k.clone())))
    .chain(snippets.iter().filter_map(|k| used.get(&("snippet".to_string(), k.clone()))))
    .chain(terms.iter().filter_map(|k| used.get(&("term".to_string(), k.clone()))))
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

  #[tokio::test]
  async fn an_entry_in_use_cannot_be_deleted_whatever_its_kind() {
    use cairn_core::{ContentStore, Placement};

    use crate::{SqlContentStore, tree::create_page};

    let db = test_db().await;
    let user = UserId::new();
    let doc = |content: serde_json::Value| -> Document { serde_json::from_value(json!({ "type": "doc", "content": content })).unwrap() };
    let variable = |key: &str| json!([{ "type": "paragraph", "content": [{ "type": "variable", "attrs": { "key": key } }] }]);

    // A page refers to `early` before the variable exists.
    let page = create_page(&db, user, None, "Page", Placement::Last).await.unwrap();
    SqlContentStore::new(db.clone()).save_draft(page.id, doc(variable("early")), None, user).await.unwrap();
    save_variable(&db, user, "early", "x").await.unwrap();
    // `nested` is used only inside a snippet; `shared` is the name of both a variable and an included snippet.
    save_variable(&db, user, "nested", "x").await.unwrap();
    save_snippet(&db, user, "holder", &doc(variable("nested"))).await.unwrap();
    save_variable(&db, user, "shared", "x").await.unwrap();
    save_snippet(&db, user, "shared", &snippet_doc(None)).await.unwrap();
    save_snippet(&db, user, "outer", &snippet_doc(Some("shared"))).await.unwrap();

    let entries = list_entries(&db).await.unwrap();
    let id = |kind: EntryKind, key: &str| entries.iter().find(|e| e.kind == kind && e.key == key).unwrap().id;

    for (kind, key, in_use) in [(EntryKind::Variable, "early", true), (EntryKind::Variable, "nested", true), (EntryKind::Variable, "shared", false)] {
      let result = delete_entry(&db, id(kind, key)).await;
      assert_eq!(matches!(result, Err(StoreError::Conflict(_))), in_use, "{key}: {result:?}");
    }
  }
}
