use std::collections::HashMap;

use jiff::Timestamp;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, sea_query::OnConflict};
use yourdocs_core::{ApiConfig, Operation, PageId, PageKind, ReleaseId, ResolvedSource, UserId};

use crate::{
  StoreError,
  entity::{api_config, release_api_spec},
  tree::list_pages,
};

/// What a release resolved one API page to. The merged document itself is in the asset store under `api-specs/<spec_hash>.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSpec {
  pub page_id: PageId,
  pub spec_hash: String,
  pub sources: Vec<ResolvedSource>,
  pub operations: Vec<Operation>,
  /// The page's settings at the build; `None` for releases made before they were recorded.
  pub config: Option<ApiConfig>,
}

/// The sources of an API page; an API page that was never configured has an empty one.
pub async fn get_api_config(db: &DatabaseConnection, page: PageId) -> Result<ApiConfig, StoreError> {
  check_api_page(db, page).await?;

  match api_config::Entity::find_by_id(page.as_uuid()).one(db).await? {
    Some(row) => into_config(&row),
    None => Ok(ApiConfig::default()),
  }
}

fn into_config(row: &api_config::Model) -> Result<ApiConfig, StoreError> {
  let bad = |e: serde_json::Error| StoreError::Invalid(format!("bad API config: {e}"));

  Ok(ApiConfig {
    sources: serde_json::from_str(&row.sources).map_err(bad)?,
    merge: serde_json::from_str(&row.merge_rules).map_err(bad)?,
    display: serde_json::from_str(&row.display).map_err(bad)?,
  })
}

/// The sources of every API page that was ever configured; pages without a row have the empty default.
pub async fn list_api_configs(db: &DatabaseConnection) -> Result<HashMap<PageId, ApiConfig>, StoreError> {
  let mut configs = HashMap::new();

  for row in api_config::Entity::find().all(db).await? {
    configs.insert(row.page_id.into(), into_config(&row)?);
  }

  Ok(configs)
}

/// Records what each API page of a release was built from, before the release goes live.
pub async fn record_release_specs(db: &DatabaseConnection, release: ReleaseId, specs: &[ReleaseSpec]) -> Result<(), StoreError> {
  if specs.is_empty() {
    return Ok(());
  }

  let now = Timestamp::now().to_string();
  let bad = |e: serde_json::Error| StoreError::Other(e.into());
  let mut rows = Vec::with_capacity(specs.len());
  for spec in specs {
    rows.push(release_api_spec::ActiveModel {
      release_id: Set(release.as_uuid()),
      page_id: Set(spec.page_id.as_uuid()),
      spec_hash: Set(spec.spec_hash.clone()),
      resolved_sources: Set(serde_json::to_string(&spec.sources).map_err(bad)?),
      operations: Set(serde_json::to_string(&spec.operations).map_err(bad)?),
      created_at: Set(now.clone()),
      config: Set(spec.config.as_ref().map(serde_json::to_string).transpose().map_err(bad)?),
    });
  }
  release_api_spec::Entity::insert_many(rows).exec(db).await?;

  Ok(())
}

pub async fn release_specs(db: &DatabaseConnection, release: ReleaseId) -> Result<Vec<ReleaseSpec>, StoreError> {
  let bad = |e: serde_json::Error| StoreError::Invalid(format!("bad recorded spec: {e}"));
  let rows = release_api_spec::Entity::find().filter(release_api_spec::Column::ReleaseId.eq(release.as_uuid())).all(db).await?;
  let mut specs = Vec::with_capacity(rows.len());

  for row in rows {
    specs.push(ReleaseSpec {
      page_id: row.page_id.into(),
      spec_hash: row.spec_hash,
      sources: serde_json::from_str(&row.resolved_sources).map_err(bad)?,
      operations: serde_json::from_str(&row.operations).map_err(bad)?,
      config: row.config.as_deref().map(serde_json::from_str).transpose().map_err(bad)?,
    });
  }

  Ok(specs)
}

pub async fn save_api_config(db: &DatabaseConnection, page: PageId, config: &ApiConfig, user: UserId) -> Result<(), StoreError> {
  config.validate().map_err(StoreError::Invalid)?;
  check_api_page(db, page).await?;

  let json = |e: serde_json::Error| StoreError::Other(e.into());
  api_config::Entity::insert(api_config::ActiveModel {
    page_id: Set(page.as_uuid()),
    sources: Set(serde_json::to_string(&config.sources).map_err(json)?),
    merge_rules: Set(serde_json::to_string(&config.merge).map_err(json)?),
    display: Set(serde_json::to_string(&config.display).map_err(json)?),
    updated_by: Set(user.as_uuid()),
    updated_at: Set(Timestamp::now().to_string()),
  })
  .on_conflict(
    OnConflict::column(api_config::Column::PageId)
      .update_columns([
        api_config::Column::Sources,
        api_config::Column::MergeRules,
        api_config::Column::Display,
        api_config::Column::UpdatedBy,
        api_config::Column::UpdatedAt,
      ])
      .to_owned(),
  )
  .exec(db)
  .await?;

  Ok(())
}

async fn check_api_page(db: &DatabaseConnection, page: PageId) -> Result<(), StoreError> {
  let pages = list_pages(db).await?;
  let found = pages.iter().find(|p| p.id == page).ok_or(StoreError::NotFound)?;

  match found.kind {
    PageKind::Api => Ok(()),
    _ => Err(StoreError::Invalid("only an API reference page has sources".into())),
  }
}

#[cfg(test)]
mod tests {
  use yourdocs_core::{ApiSource, OldUrls, Placement};

  use super::*;
  use crate::{connect::test_db, create_api_page, create_page, delete_page, list_pages, move_page, restore_page, set_in_header};

  #[tokio::test]
  async fn api_pages_behave_like_pages_and_keep_their_sources() {
    let db = test_db().await;
    let user = UserId::new();

    let guide = create_page(&db, user, None, "Guide", Placement::Last).await.unwrap();
    let api = create_api_page(&db, user, None, "Orders API", Placement::Last).await.unwrap();
    assert_eq!(api.kind, PageKind::Api);
    assert!(get_api_config(&db, api.id).await.unwrap().sources.is_empty());

    let config = ApiConfig {
      sources: vec![ApiSource::Url {
        url: "https://example.com/o.yaml".into(),
        prefix: "orders".into(),
      }],
      ..Default::default()
    };
    save_api_config(&db, api.id, &config, user).await.unwrap();
    assert_eq!(get_api_config(&db, api.id).await.unwrap(), config);
    assert!(matches!(save_api_config(&db, guide.id, &config, user).await, Err(StoreError::Invalid(_))));

    // It moves, is promoted, takes children, and comes back from the trash with its sources.
    move_page(&db, api.id, Some(guide.id), Placement::Last, OldUrls::Redirect).await.unwrap();
    assert!(set_in_header(&db, api.id, true).await.unwrap().in_header);
    create_page(&db, user, Some(api.id), "Changelog", Placement::Last).await.unwrap();

    let removed = delete_page(&db, api.id).await.unwrap();
    assert_eq!(removed.len(), 2);
    restore_page(&db, api.id).await.unwrap();
    assert_eq!(list_pages(&db).await.unwrap().len(), 3);
    assert_eq!(get_api_config(&db, api.id).await.unwrap(), config);
  }
}
