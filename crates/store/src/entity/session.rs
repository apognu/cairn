use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id_hash: String,
  pub user_id: Uuid,
  pub created_at: String,
  pub last_seen_at: String,
  pub expires_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
