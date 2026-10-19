use sea_orm_migration::prelude::*;

// What the media library shows and searches, and what keeps an asset from cleanup though no page shows it. `search` is the lowercased
// title and file name, compared with LIKE, which behaves the same on both databases.
const SCHEMA: &str = r#"
ALTER TABLE "assets" ADD COLUMN "title" TEXT;
ALTER TABLE "assets" ADD COLUMN "name" TEXT;
ALTER TABLE "assets" ADD COLUMN "search" TEXT NOT NULL DEFAULT '';
ALTER TABLE "assets" ADD COLUMN "derived_from" BLOB;
ALTER TABLE "assets" ADD COLUMN "kept" INTEGER NOT NULL DEFAULT 0;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0025_asset_meta"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
