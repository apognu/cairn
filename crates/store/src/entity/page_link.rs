use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "page_links")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub source_page_id: Uuid,
  #[sea_orm(primary_key, auto_increment = false)]
  pub target_page_id: Uuid,
  #[sea_orm(primary_key, auto_increment = false)]
  pub anchor: String,
  #[sea_orm(primary_key, auto_increment = false)]
  pub kind: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
