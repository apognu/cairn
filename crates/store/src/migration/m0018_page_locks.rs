use sea_orm_migration::prelude::*;

// One row per locked page. `expires_at` is milliseconds since the epoch, not RFC 3339 text like the other timestamps: the acquire and
// renew statements compare it in SQL, and text with a varying number of fractional digits does not sort as time does.
const SCHEMA: &str = r#"
CREATE TABLE "page_locks" (
  "page_id" BLOB NOT NULL PRIMARY KEY REFERENCES "pages" ("id"),
  "user_id" BLOB NOT NULL,
  "token" BLOB NOT NULL,
  "acquired_at" TEXT NOT NULL,
  "renewed_at" TEXT NOT NULL,
  "expires_at" INTEGER NOT NULL
) STRICT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0018_page_locks"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
