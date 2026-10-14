use sea_orm_migration::prelude::*;

// When the holder last did something (epoch milliseconds, compared in SQL like `expires_at`): the editor renews the lease while its
// author is around but idle, and only idle holders can be taken over without forcing it. 0 for locks taken before: they count as idle.
const SCHEMA: &str = r#"
ALTER TABLE "page_locks" ADD COLUMN "active_at" INTEGER NOT NULL DEFAULT 0;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0019_lock_activity"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
