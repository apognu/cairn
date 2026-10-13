use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

mod m0001_initial;
mod m0002_page_links;
mod m0003_custom_urls;
mod m0004_header;
mod m0005_backlinks;
mod m0006_description;
mod m0007_library;
mod m0008_api;
mod m0009_release_config;
mod m0010_users;
mod m0011_sessions;

struct Migrator;

impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(m0001_initial::Migration),
      Box::new(m0002_page_links::Migration),
      Box::new(m0003_custom_urls::Migration),
      Box::new(m0004_header::Migration),
      Box::new(m0005_backlinks::Migration),
      Box::new(m0006_description::Migration),
      Box::new(m0007_library::Migration),
      Box::new(m0008_api::Migration),
      Box::new(m0009_release_config::Migration),
      Box::new(m0010_users::Migration),
      Box::new(m0011_sessions::Migration),
    ]
  }
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
  Migrator::up(db, None).await
}
