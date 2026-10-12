use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

mod m0001_initial;

struct Migrator;

impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m0001_initial::Migration)]
  }
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
  Migrator::up(db, None).await
}
