use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend};
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
mod m0012_api_tokens;
mod m0013_release_preview;
mod m0014_content_hash;
mod m0015_revision_description;
mod m0016_render_keys;
mod m0017_unique_slugs;

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
      Box::new(m0012_api_tokens::Migration),
      Box::new(m0013_release_preview::Migration),
      Box::new(m0014_content_hash::Migration),
      Box::new(m0015_revision_description::Migration),
      Box::new(m0016_render_keys::Migration),
      Box::new(m0017_unique_slugs::Migration),
    ]
  }
}

/// Runs a migration written for SQLite on the connected database. The schemas are written once; for Postgres the few spellings that differ
/// are translated: IDs are 16-byte `BLOB`s on SQLite and `UUID`s on Postgres, `INTEGER` is 64 bits there, and `STRICT` has no meaning.
pub(super) async fn run(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
  let connection = manager.get_connection();
  let sql = match connection.get_database_backend() {
    DbBackend::Postgres => postgres(sql),
    _ => sql.to_string(),
  };
  connection.execute_unprepared(&sql).await?;

  Ok(())
}

fn postgres(sql: &str) -> String {
  sql
    .replace(") STRICT;", ");")
    // The one flag: written as 0/1 for SQLite, a real boolean for Postgres, which is what the entity reads.
    .replace("\"in_header\" INTEGER NOT NULL DEFAULT 0", "\"in_header\" BOOLEAN NOT NULL DEFAULT FALSE")
    .replace("BLOB", "UUID")
    .replace("INTEGER", "BIGINT")
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
  Migrator::up(db, None).await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sqlite_schemas_are_translated_for_postgres() {
    let sql = "CREATE TABLE \"t\" (\n  \"id\" BLOB NOT NULL PRIMARY KEY,\n  \"n\" INTEGER,\n  \"in_header\" INTEGER NOT NULL DEFAULT 0\n) STRICT;";

    let translated = postgres(sql);

    assert!(translated.contains("\"id\" UUID NOT NULL") && translated.contains("\"n\" BIGINT,"));
    assert!(translated.contains("\"in_header\" BOOLEAN NOT NULL DEFAULT FALSE"));
    assert!(!translated.contains("STRICT") && !translated.contains("BLOB"));
  }
}
