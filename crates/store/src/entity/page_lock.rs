use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "page_locks")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub page_id: Uuid,
  pub user_id: Uuid,
  pub token: Uuid,
  pub acquired_at: String,
  pub renewed_at: String,
  pub expires_at: i64,
  pub active_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
