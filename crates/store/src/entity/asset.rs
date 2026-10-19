use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "assets")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id: Uuid,
  pub sha256: String,
  pub mime: String,
  pub width: Option<i64>,
  pub height: Option<i64>,
  pub bytes: i64,
  pub variants: String,
  pub placeholder: Option<String>,
  pub uploaded_by: Uuid,
  pub created_at: String,
  pub title: Option<String>,
  pub name: Option<String>,
  pub search: String,
  pub derived_from: Option<Uuid>,
  pub kept: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
