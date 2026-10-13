use jiff::Timestamp;
use sea_orm::{DatabaseConnection, EntityTrait, Set, sea_query::OnConflict};
use yourdocs_core::{ApiConfig, PageId, PageKind, UserId};

use crate::{StoreError, entity::api_config, tree::list_pages};

/// The sources of an API page; an API page that was never configured has an empty one.
pub async fn get_api_config(db: &DatabaseConnection, page: PageId) -> Result<ApiConfig, StoreError> {
  check_api_page(db, page).await?;

  let Some(row) = api_config::Entity::find_by_id(page.as_uuid()).one(db).await? else {
    return Ok(ApiConfig::default());
  };
  let bad = |e: serde_json::Error| StoreError::Invalid(format!("bad API config: {e}"));

  Ok(ApiConfig {
    sources: serde_json::from_str(&row.sources).map_err(bad)?,
    merge: serde_json::from_str(&row.merge_rules).map_err(bad)?,
    display: serde_json::from_str(&row.display).map_err(bad)?,
  })
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
