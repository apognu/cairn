use sea_orm_migration::prelude::*;

// Variables, snippets and glossary terms; `key` is the variable or snippet name, or the term itself.
const SCHEMA: &str = r#"
CREATE TABLE "library_entries" (
  "id" BLOB NOT NULL PRIMARY KEY,
  "kind" TEXT NOT NULL,
  "key" TEXT NOT NULL,
  "aliases" TEXT NOT NULL DEFAULT '[]',
  "text_value" TEXT,
  "doc" TEXT,
  "link_page_id" BLOB REFERENCES "pages" ("id"),
  "updated_by" BLOB NOT NULL,
  "updated_at" TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX "library_entries_key" ON "library_entries" ("kind", "key");

CREATE TABLE "entry_usages" (
  "entry_id" BLOB NOT NULL REFERENCES "library_entries" ("id") ON DELETE CASCADE,
  "page_id" BLOB NOT NULL REFERENCES "pages" ("id"),
  PRIMARY KEY ("entry_id", "page_id")
) STRICT;
CREATE INDEX "entry_usages_page" ON "entry_usages" ("page_id");
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0007_library"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
