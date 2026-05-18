//! Adds soft-delete support for text messages.

use sea_orm_migration::prelude::*;

/// Adds `deleted_at` to `text_messages`.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TextMessages::Table)
                    .add_column(
                        ColumnDef::new(TextMessages::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TextMessages::Table)
                    .drop_column(TextMessages::DeletedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TextMessages {
    Table,
    DeletedAt,
}
