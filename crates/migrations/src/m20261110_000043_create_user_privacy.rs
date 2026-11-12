//! Настройки приватности и блокировки пользователей.

use sea_orm_migration::prelude::*;

/// Создаёт настройки входящих контактов и историю последней блокировки пары.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserPrivacySettings::Table)
                    .col(
                        ColumnDef::new(UserPrivacySettings::UserId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserPrivacySettings::DirectMessages)
                            .string_len(16)
                            .not_null()
                            .default("everyone"),
                    )
                    .col(
                        ColumnDef::new(UserPrivacySettings::DirectCalls)
                            .string_len(16)
                            .not_null()
                            .default("friends"),
                    )
                    .col(
                        ColumnDef::new(UserPrivacySettings::VoiceRoomInvites)
                            .string_len(16)
                            .not_null()
                            .default("friends"),
                    )
                    .col(
                        ColumnDef::new(UserPrivacySettings::DirectMessagesRestrictedSince)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserPrivacySettings::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_privacy_settings_user")
                            .from(UserPrivacySettings::Table, UserPrivacySettings::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(UserBlocks::Table)
                    .col(
                        ColumnDef::new(UserBlocks::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserBlocks::BlockerUserId).uuid().not_null())
                    .col(ColumnDef::new(UserBlocks::BlockedUserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserBlocks::BlockedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserBlocks::UnblockedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_blocks_blocker")
                            .from(UserBlocks::Table, UserBlocks::BlockerUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_blocks_blocked")
                            .from(UserBlocks::Table, UserBlocks::BlockedUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_user_blocks_pair_unique")
                    .table(UserBlocks::Table)
                    .col(UserBlocks::BlockerUserId)
                    .col(UserBlocks::BlockedUserId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_user_blocks_blocked_user")
                    .table(UserBlocks::Table)
                    .col(UserBlocks::BlockedUserId)
                    .col(UserBlocks::UnblockedAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserBlocks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserPrivacySettings::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum UserPrivacySettings {
    Table,
    UserId,
    DirectMessages,
    DirectCalls,
    VoiceRoomInvites,
    DirectMessagesRestrictedSince,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserBlocks {
    Table,
    Id,
    BlockerUserId,
    BlockedUserId,
    BlockedAt,
    UnblockedAt,
}
