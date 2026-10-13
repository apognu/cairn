use sea_orm_migration::prelude::*;

// What a release was built with besides its pages: the site settings, and each API page's settings. A re-render of the live release uses
// them instead of what was saved since, and "what changed" compares with them. Releases made before have neither.
const SCHEMA: &str = r#"
ALTER TABLE "releases" ADD COLUMN "settings_snapshot" TEXT;
ALTER TABLE "release_api_specs" ADD COLUMN "config" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0009_release_config"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.get_connection().execute_unprepared(SCHEMA).await?;

    Ok(())
  }
}
