use sea_orm_migration::prelude::*;

// Two editors creating or moving pages at once could both pass the sibling check in Rust; the database now refuses the second. Root
// pages get their own index because NULL parents never compare equal in a unique index, on either database. Trashed pages are left out:
// their slug is free until one is restored.
const SCHEMA: &str = r#"
CREATE UNIQUE INDEX "pages_live_slug" ON "pages" ("parent_id", "slug") WHERE "deleted_at" IS NULL AND "parent_id" IS NOT NULL;
CREATE UNIQUE INDEX "pages_live_root_slug" ON "pages" ("slug") WHERE "deleted_at" IS NULL AND "parent_id" IS NULL;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0017_unique_slugs"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
