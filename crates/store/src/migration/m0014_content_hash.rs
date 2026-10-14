use sea_orm_migration::prelude::*;

// What each page's content was made of when it went out: its expanded document and what it shows of the pages and terms it uses. Comparing
// it with the current one finds pages changed by a library entry or by a page they link to. Pages of older releases have none.
const SCHEMA: &str = r#"
ALTER TABLE "release_pages" ADD COLUMN "content_hash" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0014_content_hash"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
