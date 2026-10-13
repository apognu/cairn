use sea_orm_migration::prelude::*;

// External links in the tree: kind = 'link' rows carry their target here.
const SCHEMA: &str = r#"
ALTER TABLE "pages" ADD COLUMN "external_url" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0002_page_links"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
