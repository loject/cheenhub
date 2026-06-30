//! Adds direct-message read state and monotonic message sequence numbers.

use sea_orm_migration::prelude::*;

/// Adds read checkpoints and unread counters for direct messages.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DmMessages::Table)
                    .add_column(ColumnDef::new(DmMessages::Seq).big_integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                WITH ordered AS (
                    SELECT id,
                           row_number() OVER (
                               PARTITION BY conversation_id
                               ORDER BY created_at, id
                           ) AS seq
                    FROM dm_messages
                )
                UPDATE dm_messages
                SET seq = ordered.seq
                FROM ordered
                WHERE dm_messages.id = ordered.id
                "#,
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(DmMessages::Table)
                    .modify_column(ColumnDef::new(DmMessages::Seq).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dm_messages_conversation_seq_unique")
                    .table(DmMessages::Table)
                    .col(DmMessages::ConversationId)
                    .col(DmMessages::Seq)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_dm_messages_conversation_sender_seq")
                    .table(DmMessages::Table)
                    .col(DmMessages::ConversationId)
                    .col(DmMessages::SenderUserId)
                    .col(DmMessages::Seq)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ConversationMemberState::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConversationMemberState::ConversationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationMemberState::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ConversationMemberState::LastReadMessageId).uuid())
                    .col(
                        ColumnDef::new(ConversationMemberState::LastReadSeq)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ConversationMemberState::LastReadAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(ConversationMemberState::UnreadCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ConversationMemberState::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ConversationMemberState::ConversationId)
                            .col(ConversationMemberState::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_conversation_member_state_conversation")
                            .from(
                                ConversationMemberState::Table,
                                ConversationMemberState::ConversationId,
                            )
                            .to(DmConversations::Table, DmConversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_conversation_member_state_user")
                            .from(
                                ConversationMemberState::Table,
                                ConversationMemberState::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_conversation_member_state_last_read_message")
                            .from(
                                ConversationMemberState::Table,
                                ConversationMemberState::LastReadMessageId,
                            )
                            .to(DmMessages::Table, DmMessages::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ConversationReadCheckpoints::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConversationReadCheckpoints::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ConversationReadCheckpoints::ConversationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationReadCheckpoints::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationReadCheckpoints::LastReadMessageId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationReadCheckpoints::LastReadSeq)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationReadCheckpoints::ReadAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationReadCheckpoints::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_conversation_read_checkpoints_conversation")
                            .from(
                                ConversationReadCheckpoints::Table,
                                ConversationReadCheckpoints::ConversationId,
                            )
                            .to(DmConversations::Table, DmConversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_conversation_read_checkpoints_user")
                            .from(
                                ConversationReadCheckpoints::Table,
                                ConversationReadCheckpoints::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_conversation_read_checkpoints_message")
                            .from(
                                ConversationReadCheckpoints::Table,
                                ConversationReadCheckpoints::LastReadMessageId,
                            )
                            .to(DmMessages::Table, DmMessages::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_conversation_read_checkpoints_lookup")
                    .table(ConversationReadCheckpoints::Table)
                    .col(ConversationReadCheckpoints::ConversationId)
                    .col(ConversationReadCheckpoints::UserId)
                    .col(ConversationReadCheckpoints::LastReadSeq)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ConversationReadCheckpoints::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(ConversationMemberState::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_dm_messages_conversation_sender_seq")
                    .table(DmMessages::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_dm_messages_conversation_seq_unique")
                    .table(DmMessages::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DmMessages::Table)
                    .drop_column(DmMessages::Seq)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum DmConversations {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum DmMessages {
    Table,
    Id,
    ConversationId,
    Seq,
    SenderUserId,
}

#[derive(DeriveIden)]
enum ConversationMemberState {
    Table,
    ConversationId,
    UserId,
    LastReadMessageId,
    LastReadSeq,
    LastReadAt,
    UnreadCount,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ConversationReadCheckpoints {
    Table,
    Id,
    ConversationId,
    UserId,
    LastReadMessageId,
    LastReadSeq,
    ReadAt,
    CreatedAt,
}
