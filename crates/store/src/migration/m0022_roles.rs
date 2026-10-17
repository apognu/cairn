use sea_orm_migration::prelude::*;

// Roles (M8.1). Before them everyone who signed in could do everything, so an existing install keeps its people able to edit: the
// first user to have signed in becomes admin and the others editors. New users start as viewers (set by `upsert_user`, not here).
// A token's role caps it below its owner's; NULL means the owner's.
const SCHEMA: &str = r#"
ALTER TABLE "users" ADD COLUMN "role" TEXT NOT NULL DEFAULT 'viewer';
UPDATE "users" SET "role" = 'editor';
UPDATE "users" SET "role" = 'admin' WHERE "id" = (SELECT "id" FROM "users" ORDER BY "created_at", "id" LIMIT 1);
ALTER TABLE "api_tokens" ADD COLUMN "role" TEXT;
"#;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m0022_roles"
  }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    super::run(manager, SCHEMA).await?;

    Ok(())
  }
}
