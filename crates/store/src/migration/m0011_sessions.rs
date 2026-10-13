use sea_orm_migration::prelude::*;

// Browser sessions. The cookie holds a random token; only its SHA-256 is stored, so a copy of the database cannot be replayed as a login.
const SCHEMA: &str = r#"
CREATE TABLE "sessions" (
  "id_hash" TEXT NOT NULL PRIMARY KEY,
  "user_id" BLOB NOT NULL REFERENCES "users" ("id"),
  "created_at" TEXT NOT NULL,
  "last_seen_at" TEXT NOT NULL,
  "expires_at" TEXT NOT NULL
) STRICT;

CREATE INDEX "sessions_user" ON "sessions" ("user_id");
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0011_sessions"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(SCHEMA).await?;

    Ok(())
  }
}
