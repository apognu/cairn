use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};

use axum::{
  Json,
  extract::{Path, State},
};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{ApiConfig, CoreError, Operation, PageId, ResolvedSource};
use yourdocs_store::{StoreError, get_api_config, save_api_config};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

/// How many operations a check lists; the count is always the whole.
const LISTED_OPERATIONS: usize = 200;

/// The checks running now, and the spec each page's last successful check produced (the editor's preview reads it).
#[derive(Default)]
pub struct ApiChecks {
  running: Mutex<HashSet<PageId>>,
  last: Mutex<HashMap<PageId, Arc<Vec<u8>>>>,
}

/// Held while a page's sources are being checked, so a second click does not start a second round of downloads.
struct Running<'a>(&'a ApiChecks, PageId);

impl<'a> Running<'a> {
  fn start(checks: &'a ApiChecks, page: PageId) -> Option<Self> {
    let inserted = checks.running.lock().expect("the lock is never held across a panic").insert(page);

    // Lazily: a guard built when the page was already running would release it again on drop.
    inserted.then(|| Running(checks, page))
  }
}

impl Drop for Running<'_> {
  fn drop(&mut self) {
    self.0.running.lock().expect("the lock is never held across a panic").remove(&self.1);
  }
}

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(get_config, put_config)).routes(routes!(check_config))
}

/// The sources an API reference page is built from.
#[utoipa::path(get, path = "/api/pages/{id}/api-config", operation_id = "getApiConfig", params(("id" = PageId, Path)), responses(
  (status = 200, body = ApiConfig),
  (status = 404, body = ErrorBody),
  (status = 422, body = ErrorBody, description = "not an API reference page"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_config(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<ApiConfig>, AppError> {
  Ok(Json(get_api_config(&state.db, id).await?))
}

/// Saves the sources. The specs themselves are fetched at every publish, so nothing here counts as a pending change.
#[utoipa::path(put, path = "/api/pages/{id}/api-config", operation_id = "saveApiConfig", params(("id" = PageId, Path)), request_body = ApiConfig, responses(
  (status = 200, body = ApiConfig),
  (status = 404, body = ErrorBody),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn put_config(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<PageId>, Json(config): Json<ApiConfig>) -> Result<Json<ApiConfig>, AppError> {
  save_api_config(&state.db, id, &config, user).await?;

  Ok(Json(get_api_config(&state.db, id).await?))
}

/// What fetching and merging a page's sources gave.
#[derive(Serialize, ToSchema)]
pub(crate) struct ApiCheck {
  /// Where each source was read from, and at which release.
  sources: Vec<ResolvedSource>,
  /// All operations of the merged spec.
  operations_count: usize,
  tags: Vec<String>,
  /// The first operations, for a glance.
  operations: Vec<Operation>,
  /// Why the sources could not be merged, one sentence each. Empty when they merged.
  errors: Vec<String>,
}

/// Fetches and merges the given sources (the unsaved form as it stands) the way a publish would, without creating anything.
#[utoipa::path(post, path = "/api/pages/{id}/api-config/check", operation_id = "checkApiConfig", params(("id" = PageId, Path)), request_body = ApiConfig, responses(
  (status = 200, body = ApiCheck),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "a check of this page is already running"),
  (status = 422, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn check_config(State(state): State<AppState>, Path(id): Path<PageId>, Json(config): Json<ApiConfig>) -> Result<Json<ApiCheck>, AppError> {
  // Makes sure the page exists and is an API page.
  get_api_config(&state.db, id).await?;
  config.validate().map_err(CoreError::Invalid)?;
  let Some(_running) = Running::start(&state.api_checks, id) else {
    return Err(StoreError::Conflict("the sources of this page are already being checked".into()).into());
  };

  Ok(Json(match state.publisher.check_api(&config).await {
    Ok(spec) => {
      let mut tags: Vec<String> = spec.operations.iter().flat_map(|o| o.tags.iter().cloned()).collect();
      tags.sort();
      tags.dedup();
      state.api_checks.last.lock().expect("the lock is never held across a panic").insert(id, Arc::new(spec.bytes));

      ApiCheck {
        sources: spec.sources,
        operations_count: spec.operations.len(),
        tags,
        operations: spec.operations.into_iter().take(LISTED_OPERATIONS).collect(),
        errors: Vec::new(),
      }
    }
    Err(message) => ApiCheck {
      sources: Vec::new(),
      operations_count: 0,
      tags: Vec::new(),
      operations: Vec::new(),
      errors: vec![message],
    },
  }))
}

#[cfg(test)]
mod tests {
  use yourdocs_core::{ApiSource, Placement};
  use yourdocs_store::create_api_page;

  use super::*;
  use crate::app::test_state;

  #[tokio::test]
  async fn a_check_reports_a_source_that_cannot_be_read_without_failing_the_request() {
    let state = test_state().await;
    let page = create_api_page(&state.db, yourdocs_auth::local_user_id(), None, "Orders", Placement::Last).await.unwrap();
    let config = ApiConfig {
      sources: vec![ApiSource::Url {
        url: "https://10.0.0.1/openapi.yaml".into(),
        prefix: String::new(),
      }],
      ..Default::default()
    };

    let Json(check) = check_config(State(state.clone()), Path(page.id), Json(config)).await.unwrap();

    assert!(check.sources.is_empty() && check.operations.is_empty());
    assert!(
      check.errors[0].contains("https://10.0.0.1/openapi.yaml: https://10.0.0.1/openapi.yaml is not an HTTPS address on the public internet"),
      "{:?}",
      check.errors
    );
    // The lock is released afterwards.
    assert!(Running::start(&state.api_checks, page.id).is_some());
  }

  #[tokio::test]
  async fn only_one_check_of_a_page_runs_at_a_time() {
    let state = test_state().await;
    let page = create_api_page(&state.db, yourdocs_auth::local_user_id(), None, "Orders", Placement::Last).await.unwrap();
    let _held = Running::start(&state.api_checks, page.id).unwrap();

    let busy = check_config(State(state.clone()), Path(page.id), Json(ApiConfig::default())).await;

    assert!(matches!(busy, Err(AppError::Store(StoreError::Conflict(_)))));
  }
}
