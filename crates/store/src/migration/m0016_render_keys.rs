use sea_orm_migration::prelude::*;

// The render cache entries a release's pages came from (a JSON list of asset store keys), so entries no kept release uses can be deleted.
// NULL for releases made before: their entries are unknown, and nothing is pruned while one of them is live.
const SCHEMA: &str = r#"
ALTER TABLE "releases" ADD COLUMN "render_keys" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0016_render_keys"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
