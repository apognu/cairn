use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "releases")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id: Uuid,
  pub base_release_id: Option<Uuid>,
  pub created_by: Uuid,
  pub created_at: String,
  pub tree_snapshot: String,
  pub redirects_snapshot: String,
  pub status: String,
  pub target_version_id: Option<String>,
  pub settings_snapshot: Option<String>,
  pub preview_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
