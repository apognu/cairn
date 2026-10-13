use sea_orm_migration::prelude::*;

// SeaQuery cannot emit STRICT, so the schema is raw SQL. IDs are 16-byte BLOBs, timestamps are RFC 3339 text in UTC, JSON is text.
const SCHEMA: &str = r#"
CREATE TABLE "pages" (
  "id" BLOB NOT NULL PRIMARY KEY,
  "kind" TEXT NOT NULL DEFAULT 'page',
  "parent_id" BLOB REFERENCES "pages" ("id"),
  "position" TEXT NOT NULL,
  "slug" TEXT NOT NULL,
  "title" TEXT NOT NULL,
  "created_by" BLOB NOT NULL,
  "created_at" TEXT NOT NULL,
  "updated_at" TEXT NOT NULL,
  "deleted_at" TEXT
) STRICT;
CREATE INDEX "pages_parent" ON "pages" ("parent_id", "position");

CREATE TABLE "revisions" (
  "id" BLOB NOT NULL PRIMARY KEY,
  "page_id" BLOB NOT NULL REFERENCES "pages" ("id"),
  "doc" TEXT NOT NULL,
  "title" TEXT NOT NULL,
  "slug" TEXT NOT NULL,
  "author_id" BLOB NOT NULL,
  "created_at" TEXT NOT NULL
) STRICT;
CREATE INDEX "revisions_page" ON "revisions" ("page_id", "created_at");

CREATE TABLE "page_drafts" (
  "page_id" BLOB NOT NULL PRIMARY KEY REFERENCES "pages" ("id"),
  "doc" TEXT NOT NULL,
  "base_revision_id" BLOB REFERENCES "revisions" ("id"),
  "updated_by" BLOB NOT NULL,
  "updated_at" TEXT NOT NULL
) STRICT;

CREATE TABLE "releases" (
  "id" BLOB NOT NULL PRIMARY KEY,
  "base_release_id" BLOB REFERENCES "releases" ("id"),
  "created_by" BLOB NOT NULL,
  "created_at" TEXT NOT NULL,
  "tree_snapshot" TEXT NOT NULL,
  "redirects_snapshot" TEXT NOT NULL,
  "status" TEXT NOT NULL,
  "target_version_id" TEXT
) STRICT;

CREATE TABLE "release_pages" (
  "release_id" BLOB NOT NULL REFERENCES "releases" ("id"),
  "page_id" BLOB NOT NULL REFERENCES "pages" ("id"),
  "revision_id" BLOB NOT NULL REFERENCES "revisions" ("id"),
  "output_hash" TEXT,
  PRIMARY KEY ("release_id", "page_id")
) STRICT;

CREATE TABLE "redirects" (
  "from_path" TEXT NOT NULL PRIMARY KEY,
  "to_page_id" BLOB REFERENCES "pages" ("id"),
  "to_url" TEXT,
  "source" TEXT NOT NULL,
  "created_at" TEXT NOT NULL
) STRICT;

CREATE TABLE "assets" (
  "id" BLOB NOT NULL PRIMARY KEY,
  "sha256" TEXT NOT NULL UNIQUE,
  "mime" TEXT NOT NULL,
  "width" INTEGER,
  "height" INTEGER,
  "bytes" INTEGER NOT NULL,
  "variants" TEXT NOT NULL DEFAULT '[]',
  "placeholder" TEXT,
  "uploaded_by" BLOB NOT NULL,
  "created_at" TEXT NOT NULL
) STRICT;

CREATE TABLE "settings" (
  "key" TEXT NOT NULL PRIMARY KEY,
  "value" TEXT NOT NULL
) STRICT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0001_initial"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
