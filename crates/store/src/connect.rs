use std::{path::Path, time::Duration};

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

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

/// A fresh migrated database in a throwaway directory.
#[cfg(test)]
pub(crate) async fn test_db() -> DatabaseConnection {
  let dir = std::env::temp_dir().join(format!("yourdocs-store-{}", uuid::Uuid::now_v7()));
  std::fs::create_dir_all(&dir).unwrap();

  connect(&dir.join("test.db")).await.unwrap()
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
