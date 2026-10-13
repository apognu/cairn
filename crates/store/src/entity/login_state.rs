use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "login_states")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub state: String,
  pub verifier: String,
  pub nonce: String,
  pub next: String,
  pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
