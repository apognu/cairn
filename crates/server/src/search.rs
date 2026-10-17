//! The editor's search: pages by their title, then by the words of their drafts.

use axum::{
  Json,
  extract::{Query, State},
};
use cairn_core::{PageId, excerpt, plain_text};
use cairn_store::{draft_docs, list_pages};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{app::AppState, error::AppError};

/// Enough to choose from; a longer list means the words should be more precise.
const LIMIT: usize = 20;

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(search))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct SearchQuery {
  /// Words to find, ignoring case; fewer than two characters find nothing.
  q: String,
}

#[derive(Serialize, ToSchema)]
struct SearchHit {
  page_id: PageId,
  title: String,
  /// The words around the match in the page's draft; none when the title matched.
  excerpt: Option<String>,
}

/// Pages whose title or draft has the words, titles first, at most twenty.
#[utoipa::path(get, path = "/api/search", operation_id = "searchPages", params(SearchQuery), responses(
  (status = 200, body = Vec<SearchHit>),
))]
#[instrument(skip_all)]
async fn search(State(state): State<AppState>, Query(query): Query<SearchQuery>) -> Result<Json<Vec<SearchHit>>, AppError> {
  let needle = query.q.trim().to_lowercase();
  if needle.chars().count() < 2 {
    return Ok(Json(Vec::new()));
  }

  // Drafts are read whole and searched here: the JSON column stays opaque to the database.
  let pages = list_pages(&state.db).await?;
  let docs = draft_docs(&state.db).await?;

  let (mut hits, mut in_text) = (Vec::new(), Vec::new());
  for page in pages {
    if page.title.to_lowercase().contains(&needle) {
      hits.push(SearchHit {
        page_id: page.id,
        title: page.title,
        excerpt: None,
      });
    } else if let Some(found) = docs.get(&page.id).and_then(|doc| excerpt(&plain_text(doc), &needle)) {
      in_text.push(SearchHit {
        page_id: page.id,
        title: page.title,
        excerpt: Some(found),
      });
    }
  }
  hits.extend(in_text);
  hits.truncate(LIMIT);

  Ok(Json(hits))
}
