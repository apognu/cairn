use sea_orm_migration::prelude::*;

// Pages left out of every build; their subpages are too (decided in core, not stored).
const SCHEMA: &str = r#"
ALTER TABLE "pages" ADD COLUMN "hidden" INTEGER NOT NULL DEFAULT 0;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0024_hidden"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
