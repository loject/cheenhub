//! Creates user nickname change history table.

use sea_orm_migration::prelude::*;

/// Stores successful user nickname changes.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserNicknameHistory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserNicknameHistory::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserNicknameHistory::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserNicknameHistory::SessionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserNicknameHistory::OldNickname)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserNicknameHistory::NewNickname)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserNicknameHistory::ChangedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("user_nickname_history_user_id_fkey")
                            .from(UserNicknameHistory::Table, UserNicknameHistory::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("user_nickname_history_session_id_fkey")
                            .from(UserNicknameHistory::Table, UserNicknameHistory::SessionId)
                            .to(Sessions::Table, Sessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("user_nickname_history_user_changed_at_idx")
                    .table(UserNicknameHistory::Table)
                    .col(UserNicknameHistory::UserId)
                    .col(UserNicknameHistory::ChangedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(UserNicknameHistory::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum UserNicknameHistory {
    Table,
    Id,
    UserId,
    SessionId,
    OldNickname,
    NewNickname,
    ChangedAt,
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
