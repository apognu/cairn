use sea_orm_migration::prelude::*;

// Rebuilt from the draft on every save; powers backlinks, delete warnings and broken-link checks.
const SCHEMA: &str = r#"
CREATE TABLE "page_links" (
  "source_page_id" BLOB NOT NULL REFERENCES "pages" ("id"),
  "target_page_id" BLOB NOT NULL REFERENCES "pages" ("id"),
  "anchor" TEXT NOT NULL,
  "kind" TEXT NOT NULL,
  PRIMARY KEY ("source_page_id", "target_page_id", "anchor", "kind")
) STRICT;
CREATE INDEX "page_links_target" ON "page_links" ("target_page_id");
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0005_backlinks"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(SCHEMA).await?;

    Ok(())
  }
}
