use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};

use axum::{
  Json,
  extract::{Path, State},
  http::header,
  response::IntoResponse,
};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{ApiConfig, AssetStore, CoreError, Operation, PageId, ResolvedSource};
use yourdocs_publish::ResolvedApi;
use yourdocs_store::{StoreError, get_api_config, latest_live_release, release_specs, save_api_config};

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
  last: Mutex<HashMap<PageId, Checked>>,
}

/// What a page's last successful check merged.
#[derive(Clone)]
struct Checked {
  spec: Arc<Vec<u8>>,
  operations: Vec<Operation>,
}

impl ApiChecks {
  /// The anchors of what the checks of every page merged, for the pre-publish checks to accept links to operations not published yet.
  pub(crate) fn anchors(&self) -> HashMap<PageId, HashSet<String>> {
    let last = self.last.lock().expect("the lock is never held across a panic");

    last.iter().map(|(page, checked)| (*page, checked.operations.iter().map(Operation::anchor).collect())).collect()
  }
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
  OpenApiRouter::new()
    .routes(routes!(get_config, put_config))
    .routes(routes!(check_config))
    .routes(routes!(get_spec))
    .routes(routes!(list_operations))
    .routes(routes!(scalar_js))
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

/// Saves the sources, merge rules and display settings. They are a pending change of the page until the next publish; the specs themselves
/// are fetched again at every publish and never are.
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

/// The merged spec of the page's last successful check, or, before any, of the live release: what the settings screen previews.
#[utoipa::path(get, path = "/api/pages/{id}/api-spec", operation_id = "getApiSpec", params(("id" = PageId, Path)), responses(
  (status = 200, content_type = "application/json", body = String),
  (status = 404, body = ErrorBody, description = "nothing was merged yet"),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn get_spec(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<impl IntoResponse, AppError> {
  let checked = state.api_checks.last.lock().expect("the lock is never held across a panic").get(&id).cloned();
  let bytes = match checked {
    Some(checked) => checked.spec.as_ref().clone(),
    None => {
      let live = latest_live_release(&state.db).await?.ok_or(AppError::NotFound)?;
      let spec = release_specs(&state.db, live.id).await?.into_iter().find(|s| s.page_id == id).ok_or(AppError::NotFound)?;

      state.assets.get(&ResolvedApi::store_key(&spec.spec_hash)).await?
    }
  };

  Ok(([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")], bytes))
}

/// An operation a link can point at.
#[derive(Serialize, ToSchema)]
pub(crate) struct OperationLink {
  method: String,
  path: String,
  summary: Option<String>,
  tags: Vec<String>,
  /// What a link stores, and what follows `#` in its address.
  anchor: String,
}

/// The operations of an API page that a link can point at: those of its last check, or of the live release before any. Empty when the
/// page was never resolved.
#[utoipa::path(get, path = "/api/pages/{id}/operations", operation_id = "listApiOperations", params(("id" = PageId, Path)), responses(
  (status = 200, body = Vec<OperationLink>),
  (status = 404, body = ErrorBody),
))]
#[instrument(skip_all, fields(page_id = %id))]
async fn list_operations(State(state): State<AppState>, Path(id): Path<PageId>) -> Result<Json<Vec<OperationLink>>, AppError> {
  get_api_config(&state.db, id).await?;

  let checked = state.api_checks.last.lock().expect("the lock is never held across a panic").get(&id).map(|c| c.operations.clone());
  let operations = match checked {
    Some(operations) => operations,
    None => match latest_live_release(&state.db).await? {
      Some(live) => release_specs(&state.db, live.id).await?.into_iter().find(|s| s.page_id == id).map(|s| s.operations).unwrap_or_default(),
      None => Vec::new(),
    },
  };

  Ok(Json(
    operations
      .into_iter()
      .map(|o| OperationLink {
        anchor: o.anchor(),
        method: o.method,
        path: o.path,
        summary: o.summary,
        tags: o.tags,
      })
      .collect(),
  ))
}

/// The API reference script (Scalar), for the settings screen's preview.
#[utoipa::path(get, path = "/api/scalar.js", operation_id = "getScalarScript", responses((status = 200, content_type = "text/javascript", body = String)))]
#[instrument(skip_all)]
async fn scalar_js(State(state): State<AppState>) -> impl IntoResponse {
  (
    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")],
    state.scalar_js.as_ref().clone(),
  )
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
      state.api_checks.last.lock().expect("the lock is never held across a panic").insert(
        id,
        Checked {
          spec: Arc::new(spec.bytes),
          operations: spec.operations.clone(),
        },
      );

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
