use sea_orm_migration::prelude::*;

// Where a build that is not live yet can be looked at (the target's own address for it, when it has one). Its version at the target is
// already `target_version_id`.
const SCHEMA: &str = r#"
ALTER TABLE "releases" ADD COLUMN "preview_url" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0013_release_preview"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(SCHEMA).await?;

    Ok(())
  }
}
