//! Drops the Postgres NOT NULL constraint from `users.password_hash`.

use sea_orm_migration::prelude::*;

/// Explicitly allows passwordless OAuth users in existing Postgres databases.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE users ALTER COLUMN password_hash SET NOT NULL")
            .await?;
        Ok(())
    }
}
