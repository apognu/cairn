use sea_orm_migration::prelude::*;

// How many files a release holds and their total size, as sent to the target: the publish dialog compares them with the target's limits.
// NULL for releases made before.
const SCHEMA: &str = r#"
ALTER TABLE "releases" ADD COLUMN "file_count" INTEGER;
ALTER TABLE "releases" ADD COLUMN "bytes" INTEGER;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0020_release_stats"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
