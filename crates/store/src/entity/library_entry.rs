use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "library_entries")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id: Uuid,
  pub kind: String,
  pub key: String,
  pub aliases: String,
  pub text_value: Option<String>,
  pub doc: Option<String>,
  pub link_page_id: Option<Uuid>,
  pub updated_by: Uuid,
  pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
