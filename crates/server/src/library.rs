use axum::{
  Json,
  extract::{Path, State},
};
use cairn_auth::CurrentUser;
use cairn_core::{Document, EntryKind, PageId};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new()
    .routes(routes!(list))
    .routes(routes!(put_variable))
    .routes(routes!(put_snippet))
    .routes(routes!(put_term))
    .routes(routes!(remove))
}

#[derive(Serialize, ToSchema)]
struct UsedBy {
  page_id: PageId,
  title: String,
}

/// A variable, snippet or glossary term of the library, with the pages that use it.
#[derive(Serialize, ToSchema)]
struct LibraryEntry {
  #[schema(value_type = String)]
  id: uuid::Uuid,
  kind: EntryKind,
  key: String,
  /// The value of a variable, or the definition of a term as plain text.
  text_value: Option<String>,
  /// The content of a snippet, or the definition of a term.
  doc: Option<Document>,
  /// Other words for a term.
  aliases: Vec<String>,
  /// The page that explains a term in full.
  page_id: Option<PageId>,
  used_in: Vec<UsedBy>,
}

#[derive(Deserialize, ToSchema)]
struct VariableBody {
  value: String,
}

#[derive(Deserialize, ToSchema)]
struct TermBody {
  term: String,
  /// Paragraphs of text with inline marks, at most 400 characters.
  definition: Document,
  #[serde(default)]
  aliases: Vec<String>,
  page_id: Option<PageId>,
}

#[derive(Deserialize, ToSchema)]
struct SnippetBody {
  doc: Document,
}

async fn entries(state: &AppState) -> Result<Vec<LibraryEntry>, AppError> {
  let pages = cairn_store::list_pages(&state.db).await?;

  Ok(
    cairn_store::list_entries(&state.db)
      .await?
      .into_iter()
      .map(|e| LibraryEntry {
        id: e.id,
        kind: e.kind,
        key: e.key,
        text_value: e.text_value,
        doc: e.doc,
        aliases: e.aliases,
        page_id: e.link_page_id,
        used_in: e
          .used_in
          .iter()
          .filter_map(|id| {
            Some(UsedBy {
              page_id: *id,
              title: pages.iter().find(|p| p.id == *id)?.title.clone(),
            })
          })
          .collect(),
      })
      .collect(),
  )
}

/// Every variable, snippet and glossary term.
#[utoipa::path(get, path = "/api/library", operation_id = "listLibrary", responses((status = 200, body = Vec<LibraryEntry>)))]
#[instrument(skip_all)]
async fn list(State(state): State<AppState>) -> Result<Json<Vec<LibraryEntry>>, AppError> {
  Ok(Json(entries(&state).await?))
}

/// Creates or changes a variable.
#[utoipa::path(put, path = "/api/library/variables/{key}", operation_id = "saveVariable", params(("key" = String, Path)), request_body = VariableBody, responses(
  (status = 200, body = Vec<LibraryEntry>),
  (status = 422, body = ErrorBody, description = "not a usable name"),
))]
#[instrument(skip_all, fields(key = %key))]
async fn put_variable(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(key): Path<String>, Json(body): Json<VariableBody>) -> Result<Json<Vec<LibraryEntry>>, AppError> {
  cairn_store::save_variable(&state.db, user, &key, &body.value).await?;

  Ok(Json(entries(&state).await?))
}

/// Creates or changes a snippet. One that would include itself is refused, with the chain in the message.
#[utoipa::path(put, path = "/api/library/snippets/{key}", operation_id = "saveSnippet", params(("key" = String, Path)), request_body = SnippetBody, responses(
  (status = 200, body = Vec<LibraryEntry>),
  (status = 422, body = ErrorBody, description = "not a usable name, or the snippet would include itself"),
))]
#[instrument(skip_all, fields(key = %key))]
async fn put_snippet(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(key): Path<String>, Json(body): Json<SnippetBody>) -> Result<Json<Vec<LibraryEntry>>, AppError> {
  cairn_store::save_snippet(&state.db, user, &key, &body.doc).await?;

  Ok(Json(entries(&state).await?))
}

/// Creates or changes a glossary term. A marked term shows its definition on hover and links to the generated Glossary page, which also
/// lists its aliases and links the page that explains it in full.
#[utoipa::path(put, path = "/api/library/terms", operation_id = "saveTerm", request_body = TermBody, responses(
  (status = 200, body = Vec<LibraryEntry>),
  (status = 422, body = ErrorBody, description = "empty or too long, an alias another term has, or a linked page that does not exist"),
))]
#[instrument(skip_all)]
async fn put_term(State(state): State<AppState>, CurrentUser(user): CurrentUser, Json(body): Json<TermBody>) -> Result<Json<Vec<LibraryEntry>>, AppError> {
  let input = cairn_store::TermInput {
    term: body.term,
    definition: body.definition,
    aliases: body.aliases,
    page: body.page_id,
  };
  cairn_store::save_term(&state.db, user, &input).await?;

  Ok(Json(entries(&state).await?))
}

/// Deletes an entry nobody uses.
#[utoipa::path(delete, path = "/api/library/{id}", operation_id = "deleteLibraryEntry", params(("id" = String, Path)), responses(
  (status = 200, body = Vec<LibraryEntry>),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "pages or snippets still use it"),
))]
#[instrument(skip_all, fields(id = %id))]
async fn remove(State(state): State<AppState>, Path(id): Path<uuid::Uuid>) -> Result<Json<Vec<LibraryEntry>>, AppError> {
  cairn_store::delete_entry(&state.db, id).await?;

  Ok(Json(entries(&state).await?))
}
