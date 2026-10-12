use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

mod m0001_initial;
mod m0002_page_links;
mod m0003_custom_urls;

struct Migrator;

impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m0001_initial::Migration), Box::new(m0002_page_links::Migration), Box::new(m0003_custom_urls::Migration)]
  }
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
  Migrator::up(db, None).await
}
