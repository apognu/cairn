use sea_orm_migration::prelude::*;

// Who looks after a page and when it should be checked again. Editorial bookkeeping, not content: it is not versioned or published, so it
// lives beside `pages` rather than in it. `review_by` is a calendar date, `YYYY-MM-DD`.
const SCHEMA: &str = r#"
CREATE TABLE "page_reviews" (
  "page_id" BLOB NOT NULL PRIMARY KEY REFERENCES "pages" ("id"),
  "owner_id" BLOB,
  "review_by" TEXT,
  "updated_by" BLOB NOT NULL,
  "updated_at" TEXT NOT NULL
) STRICT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0021_page_reviews"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
