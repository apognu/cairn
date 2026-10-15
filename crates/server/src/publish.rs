use std::convert::Infallible;

use axum::{
  Json,
  extract::{Path, State},
  response::{
    Sse,
    sse::{Event as SseEvent, KeepAlive},
  },
};
use cairn_auth::CurrentUser;
use cairn_core::{Event, PageId, Pending, Problem, Pulled, Release, ReleaseId, ReleaseStatus};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tracing::instrument;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
  app::AppState,
  error::{AppError, ErrorBody},
};

pub(crate) fn routes() -> OpenApiRouter<AppState> {
  OpenApiRouter::new()
    .routes(routes!(publish))
    .routes(routes!(preview))
    .routes(routes!(promote))
    .routes(routes!(releases))
    .routes(routes!(rollback))
    .routes(routes!(changes))
    .routes(routes!(checks))
    .routes(routes!(site))
    .routes(routes!(progress))
    .routes(routes!(plan))
}

#[derive(Serialize, ToSchema)]
struct PublishProgress {
  /// Files sent so far.
  sent: u64,
  /// Files this build sends: those the target does not have yet. Unchanged files are not counted.
  total: u64,
}

/// Server-sent events while a build is sent to the publish target, each a `PublishProgress` as JSON, about one per percent. Nothing comes
/// while the pages render. Builds run one at a time, so the events are those of the build running.
#[utoipa::path(get, path = "/api/publish/progress", operation_id = "publishProgress", responses(
  (status = 200, content_type = "text/event-stream", body = PublishProgress),
))]
#[instrument(skip_all)]
async fn progress(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
  let stopping = state.until_stopping();
  let stream = BroadcastStream::new(state.publisher.subscribe()).take_until(stopping).filter_map(|event| async move {
    let Ok(Event::PublishProgress { sent, total }) = event else { return None };
    SseEvent::default().json_data(PublishProgress { sent, total }).ok().map(Ok)
  });

  Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Serialize, ToSchema)]
struct SiteInfo {
  /// Where readers find the published site, without a trailing slash.
  url: String,
  /// What the editor calls the publish target ("Cloudflare Pages").
  target: String,
  /// The release live now, and who published it.
  live: Option<LiveInfo>,
  /// What the configured publish target can do; the editor hides what it cannot.
  capabilities: SiteCapabilities,
}

#[derive(Serialize, ToSchema)]
struct SiteCapabilities {
  /// Whether a build can be viewed before it goes live.
  preview: bool,
  /// The most files a release may hold on this target.
  max_files: Option<u64>,
  /// Whether that build can then be made live exactly as it is.
  promote: bool,
}

#[derive(Serialize, ToSchema)]
struct LiveInfo {
  release: Release,
  published_by: String,
}

/// Where the published site lives, for the editor's "View as reader", and what the publish dialog says of its target.
#[utoipa::path(get, path = "/api/site", operation_id = "siteInfo", responses((status = 200, body = SiteInfo)))]
#[instrument(skip_all)]
async fn site(State(state): State<AppState>) -> Result<Json<SiteInfo>, AppError> {
  let target = state.publisher.target();
  let live = match cairn_store::latest_live_release(&state.db).await? {
    Some(release) => Some(LiveInfo {
      published_by: crate::me::display_name(&state, release.created_by).await?,
      release,
    }),
    None => None,
  };

  Ok(Json(SiteInfo {
    url: state.publisher.site_url().to_string(),
    target: target.label().into(),
    live,
    capabilities: SiteCapabilities {
      preview: target.capabilities().preview,
      promote: target.capabilities().promote,
      max_files: target.capabilities().max_files.map(|n| n as u64),
    },
  }))
}

#[derive(Deserialize, ToSchema)]
struct PublishRequest {
  /// Publish only these pages (a hotfix), with what they need; every other page keeps its published version. Everything when absent.
  pages: Option<Vec<PageId>>,
}

/// Publishes the latest saved draft of every page as a new release, or of the pages picked for a hotfix.
#[utoipa::path(post, path = "/api/publish", operation_id = "publish", request_body = Option<PublishRequest>, responses(
  (status = 200, body = Release),
  (status = 422, body = ErrorBody, description = "the checks found problems or a page could not be rendered; nothing was published; the details name each page"),
))]
#[instrument(skip_all)]
async fn publish(State(state): State<AppState>, CurrentUser(user): CurrentUser, body: Option<Json<PublishRequest>>) -> Result<Json<Release>, AppError> {
  let release = match body.and_then(|Json(b)| b.pages) {
    Some(pages) => state.publisher.publish_selected(user, &pages.into_iter().collect()).await?,
    None => state.publisher.publish(user).await?,
  };

  Ok(Json(release))
}

#[derive(Deserialize, ToSchema)]
struct PlanRequest {
  pages: Vec<PageId>,
}

#[derive(Serialize, ToSchema)]
struct HotfixPlan {
  /// Pages the hotfix takes along: the new parent of a picked page, or a page left under one the hotfix deletes or moves.
  pulled: Vec<Pulled>,
  /// Links from the hotfix to pages it leaves out, which show as plain text.
  plain_links: Vec<PlainLink>,
}

#[derive(Serialize, ToSchema)]
struct PlainLink {
  source_title: String,
  target_id: PageId,
  target_title: String,
}

/// What a hotfix of these pages would also publish, and which of their links would show as plain text. Nothing is published.
#[utoipa::path(post, path = "/api/publish/plan", operation_id = "hotfixPlan", request_body = PlanRequest, responses((status = 200, body = HotfixPlan)))]
#[instrument(skip_all)]
async fn plan(State(state): State<AppState>, Json(body): Json<PlanRequest>) -> Result<Json<HotfixPlan>, AppError> {
  let plan = cairn_publish::hotfix_plan(&state.db, &body.pages.into_iter().collect()).await?;

  Ok(Json(HotfixPlan {
    pulled: plan.pulled,
    plain_links: plan
      .plain_links
      .into_iter()
      .map(|l| PlainLink {
        source_title: l.source_title,
        target_id: l.target_id,
        target_title: l.target_title,
      })
      .collect(),
  }))
}

/// Builds the latest saved drafts as a release that is not live: it can be looked at (`preview_url` when the target gives it an address of
/// its own, `/_preview/<id>/` when it is self-hosted) and then promoted. Only for targets that can do it.
#[utoipa::path(post, path = "/api/publish/preview", operation_id = "publishPreview", responses(
  (status = 200, body = Release),
  (status = 422, body = ErrorBody, description = "the target has no previews, the checks found problems, or the target would refuse the site"),
))]
#[instrument(skip_all)]
async fn preview(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> Result<Json<Release>, AppError> {
  Ok(Json(state.publisher.preview(user).await?))
}

/// Makes a preview the live site, exactly as it was built.
#[utoipa::path(post, path = "/api/releases/{id}/promote", operation_id = "promoteRelease", params(("id" = ReleaseId, Path)), responses(
  (status = 200, body = Release),
  (status = 404, body = ErrorBody),
  (status = 409, body = ErrorBody, description = "the site was published after the preview was made"),
  (status = 422, body = ErrorBody, description = "the release is not a preview"),
))]
#[instrument(skip_all)]
async fn promote(State(state): State<AppState>, _user: CurrentUser, Path(id): Path<ReleaseId>) -> Result<Json<Release>, AppError> {
  Ok(Json(state.publisher.promote(id).await?))
}

/// What the next publish would change compared with the live site, and the live URLs it would break.
#[utoipa::path(get, path = "/api/publish/changes", operation_id = "pendingChanges", responses((status = 200, body = Pending)))]
#[instrument(skip_all)]
async fn changes(State(state): State<AppState>) -> Result<Json<Pending>, AppError> {
  Ok(Json(cairn_publish::pending(&state.db).await?))
}

/// The problems that would stop a publish right now, each naming its page. An empty list means the site can be published.
#[utoipa::path(get, path = "/api/publish/checks", operation_id = "publishChecks", responses((status = 200, body = Vec<Problem>)))]
#[instrument(skip_all)]
async fn checks(State(state): State<AppState>) -> Result<Json<Vec<Problem>>, AppError> {
  Ok(Json(cairn_publish::check(&state.db, &state.pool, &state.api_checks.anchors()).await?))
}

#[derive(Serialize, ToSchema)]
struct ReleaseRow {
  release: Release,
  /// 1 for the first release that went live; stays with its release.
  number: u32,
  /// Titles of the pages that changed compared with the release it was built on: edited, new or removed.
  changes: Vec<String>,
  /// The name of whoever published it.
  published_by: String,
  /// Whether it can be rolled back to now: it was the site and is not any more.
  can_roll_back: bool,
  /// Whether rolling back gives back exactly the files that were published, rather than rendering the release again from its content.
  exact: bool,
}

/// The latest releases that were the live site, newest first, for the Releases settings and rollback.
#[utoipa::path(get, path = "/api/releases", operation_id = "listReleases", responses((status = 200, body = Vec<ReleaseRow>)))]
#[instrument(skip_all)]
async fn releases(State(state): State<AppState>) -> Result<Json<Vec<ReleaseRow>>, AppError> {
  let mut rows = Vec::new();
  for entry in cairn_store::release_log(&state.db).await? {
    let can_roll_back = entry.release.status == ReleaseStatus::Superseded;
    rows.push(ReleaseRow {
      published_by: crate::me::display_name(&state, entry.release.created_by).await?,
      exact: can_roll_back && state.publisher.rolls_back_exactly(entry.release.id).await?,
      can_roll_back,
      number: entry.number,
      changes: entry.changes,
      release: entry.release,
    });
  }

  Ok(Json(rows))
}

/// Makes an earlier release the live site again. Drafts are untouched, and what was published after it shows as pending.
#[utoipa::path(post, path = "/api/releases/{id}/rollback", operation_id = "rollbackRelease", params(("id" = ReleaseId, Path)), responses(
  (status = 200, body = Release, description = "the release that is the site now: the one asked for, or a new one rendered from its content"),
  (status = 404, body = ErrorBody),
  (status = 422, body = ErrorBody, description = "the release is the live site already, or never was the site"),
))]
#[instrument(skip_all)]
async fn rollback(State(state): State<AppState>, CurrentUser(user): CurrentUser, Path(id): Path<ReleaseId>) -> Result<Json<Release>, AppError> {
  Ok(Json(state.publisher.rollback(user, id).await?))
}
