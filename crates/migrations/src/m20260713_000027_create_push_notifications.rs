//! Таблицы установок и постоянной очереди системных push-уведомлений.

use sea_orm_migration::prelude::*;

/// Создаёт хранилище push-установок и заданий доставки.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PushInstallations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PushInstallations::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PushInstallations::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(PushInstallations::SessionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushInstallations::Platform)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(PushInstallations::Token).text().not_null())
                    .col(
                        ColumnDef::new(PushInstallations::Active)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(PushInstallations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushInstallations::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_push_installations_user")
                            .from(PushInstallations::Table, PushInstallations::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_push_installations_session")
                            .from(PushInstallations::Table, PushInstallations::SessionId)
                            .to(Sessions::Table, Sessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_push_installations_token_unique")
                    .table(PushInstallations::Table)
                    .col(PushInstallations::Token)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_push_installations_recipient")
                    .table(PushInstallations::Table)
                    .col(PushInstallations::UserId)
                    .col(PushInstallations::Active)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PushDeliveryQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PushDeliveryQueue::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PushDeliveryQueue::InstallationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDeliveryQueue::MessageId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDeliveryQueue::Payload)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDeliveryQueue::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(PushDeliveryQueue::NextAttemptAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushDeliveryQueue::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_push_delivery_installation")
                            .from(PushDeliveryQueue::Table, PushDeliveryQueue::InstallationId)
                            .to(PushInstallations::Table, PushInstallations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_push_delivery_message_installation_unique")
                    .table(PushDeliveryQueue::Table)
                    .col(PushDeliveryQueue::MessageId)
                    .col(PushDeliveryQueue::InstallationId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_push_delivery_due")
                    .table(PushDeliveryQueue::Table)
                    .col(PushDeliveryQueue::NextAttemptAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PushDeliveryQueue::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PushInstallations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PushInstallations {
    Table,
    Id,
    UserId,
    SessionId,
    Platform,
    Token,
    Active,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PushDeliveryQueue {
    Table,
    Id,
    InstallationId,
    MessageId,
    Payload,
    Attempts,
    NextAttemptAt,
    CreatedAt,
}
