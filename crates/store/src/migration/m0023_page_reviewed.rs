use sea_orm_migration::prelude::*;

// Who last marked a page as reviewed, and when (M8.6). Separate from `updated_by`, which any change of owner or date also sets.
const SCHEMA: &str = r#"
ALTER TABLE "page_reviews" ADD COLUMN "reviewed_by" BLOB;
ALTER TABLE "page_reviews" ADD COLUMN "reviewed_at" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0023_page_reviewed"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
