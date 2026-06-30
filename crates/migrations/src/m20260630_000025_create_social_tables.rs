//! Таблицы друзей и личных сообщений.

use sea_orm_migration::prelude::*;

/// Создает таблицы дружбы, личных диалогов и сообщений.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Friendships::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Friendships::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Friendships::RequesterUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Friendships::RecipientUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Friendships::UserLowId).uuid().not_null())
                    .col(ColumnDef::new(Friendships::UserHighId).uuid().not_null())
                    .col(
                        ColumnDef::new(Friendships::Status)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Friendships::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Friendships::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_friendships_requester")
                            .from(Friendships::Table, Friendships::RequesterUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_friendships_recipient")
                            .from(Friendships::Table, Friendships::RecipientUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_friendships_pair_unique")
                    .table(Friendships::Table)
                    .col(Friendships::UserLowId)
                    .col(Friendships::UserHighId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_friendships_requester_status")
                    .table(Friendships::Table)
                    .col(Friendships::RequesterUserId)
                    .col(Friendships::Status)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_friendships_recipient_status")
                    .table(Friendships::Table)
                    .col(Friendships::RecipientUserId)
                    .col(Friendships::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DmConversations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DmConversations::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DmConversations::UserLowId).uuid().not_null())
                    .col(
                        ColumnDef::new(DmConversations::UserHighId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DmConversations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DmConversations::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_dm_conversations_user_low")
                            .from(DmConversations::Table, DmConversations::UserLowId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_dm_conversations_user_high")
                            .from(DmConversations::Table, DmConversations::UserHighId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dm_conversations_pair_unique")
                    .table(DmConversations::Table)
                    .col(DmConversations::UserLowId)
                    .col(DmConversations::UserHighId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_dm_conversations_updated")
                    .table(DmConversations::Table)
                    .col(DmConversations::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DmMessages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DmMessages::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DmMessages::ConversationId).uuid().not_null())
                    .col(ColumnDef::new(DmMessages::SenderUserId).uuid().not_null())
                    .col(ColumnDef::new(DmMessages::Body).text().not_null())
                    .col(
                        ColumnDef::new(DmMessages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DmMessages::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DmMessages::DeletedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_dm_messages_conversation")
                            .from(DmMessages::Table, DmMessages::ConversationId)
                            .to(DmConversations::Table, DmConversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_dm_messages_sender")
                            .from(DmMessages::Table, DmMessages::SenderUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dm_messages_conversation_created")
                    .table(DmMessages::Table)
                    .col(DmMessages::ConversationId)
                    .col(DmMessages::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DmMessages::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DmConversations::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Friendships::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Friendships {
    Table,
    Id,
    RequesterUserId,
    RecipientUserId,
    UserLowId,
    UserHighId,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum DmConversations {
    Table,
    Id,
    UserLowId,
    UserHighId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum DmMessages {
    Table,
    Id,
    ConversationId,
    SenderUserId,
    Body,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
