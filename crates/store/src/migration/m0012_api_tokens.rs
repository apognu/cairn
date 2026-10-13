use sea_orm_migration::prelude::*;

// Tokens for scripts and CI. Like sessions, only the hash of the secret is stored; the secret is shown once, at creation.
const SCHEMA: &str = r#"
CREATE TABLE "api_tokens" (
  "id" BLOB NOT NULL PRIMARY KEY,
  "user_id" BLOB NOT NULL REFERENCES "users" ("id"),
  "name" TEXT NOT NULL,
  "token_hash" TEXT NOT NULL UNIQUE,
  "created_at" TEXT NOT NULL,
  "last_used_at" TEXT
) STRICT;

CREATE INDEX "api_tokens_user" ON "api_tokens" ("user_id");
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0012_api_tokens"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
