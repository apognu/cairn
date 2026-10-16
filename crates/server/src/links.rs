use std::collections::HashSet;

use axum::{
  Json,
  extract::{Path, State},
};
use cairn_core::{EntryKind, LinkKind, Page, PageId};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(backlinks)).routes(routes!(impact))
}

/// A page that links to another.
#[derive(Serialize, ToSchema)]
struct Backlink {
  source_id: PageId,
  source_title: String,
  /// The page it links to (one of the pages below the one asked about, for a delete).
  target_title: String,
  kind: LinkKind,
}

/// A glossary term whose full explanation is the page.
#[derive(Serialize, ToSchema)]
struct TermLink {
  term: String,
  /// The page it points to (one of the pages below the one asked about, for a delete).
  target_title: String,
}

#[derive(Serialize, ToSchema)]
struct Backlinks {
  linked_from: Vec<Backlink>,
  /// Glossary terms that send readers to the page for the full explanation.
  terms: Vec<TermLink>,
}

#[derive(Serialize, ToSchema)]
struct Impact {
  /// How many pages go with it.
  subpages: usize,
  /// Pages outside the deleted ones that link to any of them.
  linked_from: Vec<Backlink>,
  /// Glossary terms that lose their link to the full explanation.
  terms: Vec<TermLink>,
}

/// The pages `id` and everything below it.
fn subtree(pages: &[Page], id: PageId) -> HashSet<PageId> {
  let mut found = HashSet::from([id]);

  // Parents come before children in no particular order, so repeat until nothing new is added.
  loop {
    let before = found.len();
    let more: Vec<PageId> = pages.iter().filter(|p| p.parent_id.is_some_and(|parent| found.contains(&parent))).map(|p| p.id).collect();
    found.extend(more);
    if found.len() == before {
      return found;
    }
  }
}

async fn links_to(state: &AppState, targets: &HashSet<PageId>, pages: &[Page]) -> Result<Vec<Backlink>, AppError> {
  let title = |id: PageId| pages.iter().find(|p| p.id == id).map(|p| p.title.clone());

  Ok(
    cairn_store::all_links(&state.db)
      .await?
      .into_iter()
      .filter(|l| targets.contains(&l.target) && !targets.contains(&l.source))
      .filter_map(|l| {
        Some(Backlink {
          source_id: l.source,
          source_title: title(l.source)?,
          target_title: title(l.target)?,
          kind: l.kind,
        })
      })
      .collect(),
  )
}

async fn terms_pointing_to(state: &AppState, targets: &HashSet<PageId>, pages: &[Page]) -> Result<Vec<TermLink>, AppError> {
  let title = |id: PageId| pages.iter().find(|p| p.id == id).map(|p| p.title.clone());

  Ok(
    cairn_store::list_entries(&state.db)
      .await?
      .into_iter()
      .filter(|e| e.kind == EntryKind::Term)
      .filter_map(|e| {
        let target = e.link_page_id.filter(|id| targets.contains(id))?;
        Some(TermLink {
          term: e.key,
          target_title: title(target)?,
        })
      })
      .collect(),
  )
}

/// The pages that link to this one.
#[utoipa::path(get, path = "/api/pages/{id}/backlinks", operation_id = "getBacklinks", params(("id" = PageId, Path)), responses(
  (status = 200, body = Backlinks),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn backlinks(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<Backlinks>, AppError> {
  let pages = cairn_store::list_pages(&state.db).await?;
  if !pages.iter().any(|p| p.id == id) {
    return Err(AppError::NotFound);
  }

  let target = HashSet::from([id]);

  Ok(Json(Backlinks {
    linked_from: links_to(&state, &target, &pages).await?,
    terms: terms_pointing_to(&state, &target, &pages).await?,
  }))
}

/// What deleting a page would affect: its subpages and every page that links into them.
#[utoipa::path(get, path = "/api/pages/{id}/impact", operation_id = "getDeleteImpact", params(("id" = PageId, Path)), responses(
  (status = 200, body = Impact),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn impact(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<Impact>, AppError> {
  let pages = cairn_store::list_pages(&state.db).await?;
  if !pages.iter().any(|p| p.id == id) {
    return Err(AppError::NotFound);
  }

  let doomed = subtree(&pages, id);
  Ok(Json(Impact {
    subpages: doomed.len() - 1,
    linked_from: links_to(&state, &doomed, &pages).await?,
    terms: terms_pointing_to(&state, &doomed, &pages).await?,
  }))
}
