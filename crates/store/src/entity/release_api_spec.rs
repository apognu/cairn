use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "release_api_specs")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub release_id: Uuid,
  #[sea_orm(primary_key, auto_increment = false)]
  pub page_id: Uuid,
  pub spec_hash: String,
  pub resolved_sources: String,
  pub operations: String,
  pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
