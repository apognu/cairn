use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "pages")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id: Uuid,
  pub kind: String,
  pub parent_id: Option<Uuid>,
  pub position: String,
  pub slug: String,
  pub title: String,
  pub external_url: Option<String>,
  pub path_override: Option<String>,
  pub in_header: bool,
  pub hidden: bool,
  pub header_position: Option<String>,
  pub description: Option<String>,
  pub created_by: Uuid,
  pub created_at: String,
  pub updated_at: String,
  pub deleted_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
