use sea_orm_migration::prelude::*;

// A short summary of a page: shown on link cards and in search results and link previews.
const SCHEMA: &str = r#"
ALTER TABLE "pages" ADD COLUMN "description" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0006_description"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(SCHEMA).await?;

    Ok(())
  }
}
