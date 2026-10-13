use sea_orm_migration::prelude::*;

// People who signed in, keyed by the identity provider and its subject (an email can change, a subject cannot), and the one-time state of
// a login in progress. Rows written before (`created_by`, `updated_by`) hold a user ID without pointing at this table: the local user of
// `dev` is not a row.
const SCHEMA: &str = r#"
CREATE TABLE "users" (
  "id" BLOB NOT NULL PRIMARY KEY,
  "issuer" TEXT NOT NULL,
  "subject" TEXT NOT NULL,
  "email" TEXT NOT NULL,
  "name" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  "last_login_at" TEXT NOT NULL,
  UNIQUE ("issuer", "subject")
) STRICT;

CREATE TABLE "login_states" (
  "state" TEXT NOT NULL PRIMARY KEY,
  "verifier" TEXT NOT NULL,
  "nonce" TEXT NOT NULL,
  "next" TEXT NOT NULL,
  "created_at" TEXT NOT NULL
) STRICT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0010_users"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(SCHEMA).await?;

    Ok(())
  }
}
