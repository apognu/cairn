use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "page_reviews")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub page_id: Uuid,
  pub owner_id: Option<Uuid>,
  pub review_by: Option<String>,
  pub updated_by: Uuid,
  pub updated_at: String,
  pub reviewed_by: Option<Uuid>,
  pub reviewed_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
