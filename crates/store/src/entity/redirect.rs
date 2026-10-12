use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "redirects")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub from_path: String,
  pub to_page_id: Option<Uuid>,
  pub to_url: Option<String>,
  pub source: String,
  pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
