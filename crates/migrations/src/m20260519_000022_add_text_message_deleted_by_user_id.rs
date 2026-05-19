//! Adds deleted_by_user_id to text_messages for moderation audit.

use sea_orm_migration::prelude::*;

/// Adds `deleted_by_user_id` to `text_messages`.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TextMessages::Table)
                    .add_column(ColumnDef::new(TextMessages::DeletedByUserId).uuid().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TextMessages::Table)
                    .drop_column(TextMessages::DeletedByUserId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TextMessages {
    Table,
    DeletedByUserId,
}
