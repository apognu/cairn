use sea_orm_migration::prelude::*;

// A fixed URL that replaces the one derived from the tree.
const SCHEMA: &str = r#"
ALTER TABLE "pages" ADD COLUMN "path_override" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0003_custom_urls"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
