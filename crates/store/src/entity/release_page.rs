use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "release_pages")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub release_id: Uuid,
  #[sea_orm(primary_key, auto_increment = false)]
  pub page_id: Uuid,
  pub revision_id: Uuid,
  pub output_hash: Option<String>,
  pub content_hash: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
