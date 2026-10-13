use sea_orm_migration::prelude::*;

// The sources of an API reference page, and what each release resolved them to (for rollback and for the next "what changed").
const SCHEMA: &str = r#"
CREATE TABLE "api_configs" (
  "page_id" BLOB NOT NULL PRIMARY KEY REFERENCES "pages" ("id"),
  "sources" TEXT NOT NULL DEFAULT '[]',
  "merge_rules" TEXT NOT NULL DEFAULT '{}',
  "display" TEXT NOT NULL DEFAULT '{}',
  "updated_by" BLOB NOT NULL,
  "updated_at" TEXT NOT NULL
) STRICT;

CREATE TABLE "release_api_specs" (
  "release_id" BLOB NOT NULL REFERENCES "releases" ("id"),
  "page_id" BLOB NOT NULL REFERENCES "pages" ("id"),
  "spec_hash" TEXT NOT NULL,
  "resolved_sources" TEXT NOT NULL,
  "operations" TEXT NOT NULL,
  "created_at" TEXT NOT NULL,
  PRIMARY KEY ("release_id", "page_id")
) STRICT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0008_api"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
