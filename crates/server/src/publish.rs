use axum::{Json, extract::State};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yourdocs_auth::CurrentUser;
use yourdocs_core::{Pending, Problem, Release};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new().routes(routes!(publish)).routes(routes!(changes)).routes(routes!(checks)).routes(routes!(site))
}

#[derive(Serialize, ToSchema)]
struct SiteInfo {
  /// Where readers find the published site, without a trailing slash.
  url: String,
  /// What the configured publish target can do; the editor hides what it cannot.
  capabilities: SiteCapabilities,
}

#[derive(Serialize, ToSchema)]
struct SiteCapabilities {
  /// Whether a build can be viewed before it goes live.
  preview: bool,
}

/// Where the published site lives, for the editor's "View as reader".
#[utoipa::path(get, path = "/api/site", operation_id = "siteInfo", responses((status = 200, body = SiteInfo)))]
#[instrument(skip_all)]
async fn site(State(state): State<AppState>) -> Json<SiteInfo> {
  Json(SiteInfo {
    url: state.publisher.site_url().to_string(),
    capabilities: SiteCapabilities {
      preview: state.publisher.target().capabilities().preview,
    },
  })
}

/// Publishes the latest saved draft of every page as a new release.
#[utoipa::path(post, path = "/api/publish", operation_id = "publish", responses(
  (status = 200, body = Release),
  (status = 422, body = ErrorBody, description = "the checks found problems or a page could not be rendered; nothing was published; the details name each page"),
))]
#[instrument(skip_all)]
async fn publish(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Result<Json<Release>, AppError> {
  Ok(Json(state.publisher.publish(user).await?))
}

/// What the next publish would change compared with the live site, and the live URLs it would break.
#[utoipa::path(get, path = "/api/publish/changes", operation_id = "pendingChanges", responses((status = 200, body = Pending)))]
#[instrument(skip_all)]
async fn changes(State(state): State<AppState>) -> Result<Json<Pending>, AppError> {
  Ok(Json(yourdocs_publish::pending(&state.db).await?))
}

/// The problems that would stop a publish right now, each naming its page. An empty list means the site can be published.
#[utoipa::path(get, path = "/api/publish/checks", operation_id = "publishChecks", responses((status = 200, body = Vec<Problem>)))]
#[instrument(skip_all)]
async fn checks(State(state): State<AppState>) -> Result<Json<Vec<Problem>>, AppError> {
  Ok(Json(yourdocs_publish::check(&state.db, &state.pool, &state.api_checks.anchors()).await?))
}
