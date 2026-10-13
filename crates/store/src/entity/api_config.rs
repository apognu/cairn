use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_configs")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub page_id: Uuid,
  pub sources: String,
  pub merge_rules: String,
  pub display: String,
  pub updated_by: Uuid,
  pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
