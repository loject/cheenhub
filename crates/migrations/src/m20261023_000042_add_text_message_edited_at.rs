//! Добавляет время последней редакции текста сообщений комнат.

use sea_orm_migration::prelude::*;

/// Добавляет необязательную метку `edited_at` в `text_messages`.
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
                        ColumnDef::new(TextMessages::EditedAt)
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
                    .drop_column(TextMessages::EditedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TextMessages {
    Table,
    EditedAt,
}
