use sea_orm_migration::prelude::*;

// Pages promoted to the site header; `header_position` is a fractional index like `position`.
const SCHEMA: &str = r#"
ALTER TABLE "pages" ADD COLUMN "in_header" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "pages" ADD COLUMN "header_position" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0004_header"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
