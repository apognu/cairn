use std::collections::HashSet;

use axum::{
  Json,
  extract::{Path, State},
};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_core::{LinkKind, Page, PageId};

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

#[derive(Serialize, ToSchema)]
struct Backlinks {
  linked_from: Vec<Backlink>,
}

#[derive(Serialize, ToSchema)]
struct Impact {
  /// How many pages go with it.
  subpages: usize,
  /// Pages outside the deleted ones that link to any of them.
  linked_from: Vec<Backlink>,
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
    yourdocs_store::all_links(&state.db)
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

/// The pages that link to this one.
#[utoipa::path(get, path = "/api/pages/{id}/backlinks", operation_id = "getBacklinks", params(("id" = PageId, Path)), responses(
  (status = 200, body = Backlinks),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn backlinks(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<Backlinks>, AppError> {
  let pages = yourdocs_store::list_pages(&state.db).await?;
  if !pages.iter().any(|p| p.id == id) {
    return Err(AppError::NotFound);
  }

  Ok(Json(Backlinks {
    linked_from: links_to(&state, &HashSet::from([id]), &pages).await?,
  }))
}

/// What deleting a page would affect: its subpages and every page that links into them.
#[utoipa::path(get, path = "/api/pages/{id}/impact", operation_id = "getDeleteImpact", params(("id" = PageId, Path)), responses(
  (status = 200, body = Impact),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn impact(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<Impact>, AppError> {
  let pages = yourdocs_store::list_pages(&state.db).await?;
  if !pages.iter().any(|p| p.id == id) {
    return Err(AppError::NotFound);
  }

  let doomed = subtree(&pages, id);
  let linked_from = links_to(&state, &doomed, &pages).await?;

  Ok(Json(Impact {
    subpages: doomed.len() - 1,
    linked_from,
  }))
}
