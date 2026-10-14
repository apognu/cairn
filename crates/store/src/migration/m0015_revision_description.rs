use sea_orm_migration::prelude::*;

// The description a page had when each version was saved, so a restore can bring it back. Empty text means it had none; NULL means the
// version was saved before descriptions were recorded.
const SCHEMA: &str = r#"
ALTER TABLE "revisions" ADD COLUMN "description" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0015_revision_description"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
