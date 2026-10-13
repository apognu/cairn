use std::{path::Path, time::Duration};

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
#[cfg(any(test, feature = "testing"))]
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use crate::migration;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
  #[error("not found")]
  NotFound,
  #[error("conflict: {0}")]
  Conflict(String),
  #[error("invalid: {0}")]
  Invalid(String),
  #[error(transparent)]
  Db(#[from] DbErr),
  #[error(transparent)]
  Other(#[from] anyhow::Error),
}

/// Opens (creating if needed) the SQLite file at `path` and applies pending migrations.
pub async fn connect(path: &Path) -> Result<DatabaseConnection, StoreError> {
  let url = format!("sqlite://{}?mode=rwc", path.display());
  let mut options = ConnectOptions::new(url);

  // WAL lets readers run during a write, and the pool's connections each need the pragmas, so they are set on the connect options.
  options.map_sqlx_sqlite_opts(|opts| opts.journal_mode(sea_orm::sqlx::sqlite::SqliteJournalMode::Wal).foreign_keys(true).busy_timeout(Duration::from_secs(5)));

  let db = Database::connect(options).await?;
  migration::migrate(&db).await?;

  Ok(db)
}

/// Opens the database a URL names (`sqlite://…` or `postgres://…`) and applies pending migrations.
pub async fn connect_url(url: &str) -> Result<DatabaseConnection, StoreError> {
  let mut options = ConnectOptions::new(url);
  options.max_connections(10).connect_timeout(Duration::from_secs(10));
  // The same on both: WAL, foreign keys and a busy timeout are SQLite's (see `connect`).
  options.map_sqlx_sqlite_opts(|opts| opts.journal_mode(sea_orm::sqlx::sqlite::SqliteJournalMode::Wal).foreign_keys(true).busy_timeout(Duration::from_secs(5)));

  let db = Database::connect(options).await?;
  migration::migrate(&db).await?;

  Ok(db)
}

/// A fresh migrated database for a test: a file in `dir`, or when `TEST_DATABASE_URL` names a Postgres server, a new database on it
/// (they are not dropped; the server is a throwaway one).
#[cfg(any(test, feature = "testing"))]
pub async fn connect_for_tests(dir: &Path) -> DatabaseConnection {
  let Ok(server) = std::env::var("TEST_DATABASE_URL") else {
    return connect(&dir.join("test.db")).await.expect("a test database opens");
  };

  let name = format!("yd_test_{}", uuid::Uuid::now_v7().simple());
  let admin = Database::connect(&server).await.expect("TEST_DATABASE_URL names a reachable Postgres");
  admin
    .execute_raw(Statement::from_string(DbBackend::Postgres, format!("CREATE DATABASE {name}")))
    .await
    .expect("a test database is created");

  // `postgres://user:pass@host:port/postgres` becomes the same with the new database.
  let (base, _) = server.rsplit_once('/').expect("TEST_DATABASE_URL ends with a database name");
  connect_url(&format!("{base}/{name}")).await.expect("a test database opens")
}

/// A fresh migrated database in a throwaway directory.
#[cfg(test)]
pub(crate) async fn test_db() -> DatabaseConnection {
  let dir = std::env::temp_dir().join(format!("yourdocs-store-{}", uuid::Uuid::now_v7()));
  std::fs::create_dir_all(&dir).unwrap();

  connect_for_tests(&dir).await
}

#[cfg(test)]
mod tests {
  use sea_orm::{ConnectionTrait, Statement};

  use super::*;

  #[tokio::test]
  async fn migrates_a_fresh_file_and_a_second_run_changes_nothing() {
    let dir = std::env::temp_dir().join(format!("yourdocs-store-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.db");

    let db = connect(&path).await.unwrap();
    let before = schema(&db).await;
    drop(db);

    let db = connect(&path).await.unwrap();
    assert_eq!(schema(&db).await, before);
    assert!(before.iter().any(|sql| sql.contains("CREATE TABLE \"pages\"") && sql.contains("STRICT")));

    let foreign_keys = db.query_one_raw(Statement::from_string(db.get_database_backend(), "PRAGMA foreign_keys")).await.unwrap().unwrap();
    assert_eq!(foreign_keys.try_get_by_index::<i64>(0).unwrap(), 1);

    std::fs::remove_dir_all(dir).ok();
  }

  async fn schema(db: &DatabaseConnection) -> Vec<String> {
    let rows = db
      .query_all_raw(Statement::from_string(db.get_database_backend(), "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name"))
      .await
      .unwrap();

    rows.iter().map(|row| row.try_get_by_index(0).unwrap()).collect()
  }
}
