//! Text chat attachment table.

use sea_orm_migration::prelude::*;

/// Creates the text_chat_attachments table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TextChatAttachments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TextChatAttachments::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::ServerId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::RoomId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::UploaderUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TextChatAttachments::MessageId).uuid())
                    .col(
                        ColumnDef::new(TextChatAttachments::Bucket)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::ObjectKey)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::ContentType)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::ByteSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::Width)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::Height)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextChatAttachments::Sha256)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(ColumnDef::new(TextChatAttachments::OriginalFilename).string_len(255))
                    .col(
                        ColumnDef::new(TextChatAttachments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_chat_attachments_server")
                            .from(TextChatAttachments::Table, TextChatAttachments::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_chat_attachments_room")
                            .from(TextChatAttachments::Table, TextChatAttachments::RoomId)
                            .to(ServerRooms::Table, ServerRooms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_chat_attachments_uploader")
                            .from(
                                TextChatAttachments::Table,
                                TextChatAttachments::UploaderUserId,
                            )
                            .to(Users::Table, Users::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_chat_attachments_message")
                            .from(TextChatAttachments::Table, TextChatAttachments::MessageId)
                            .to(TextMessages::Table, TextMessages::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_text_chat_attachments_room_created")
                    .table(TextChatAttachments::Table)
                    .col(TextChatAttachments::RoomId)
                    .col(TextChatAttachments::CreatedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_text_chat_attachments_message")
                    .table(TextChatAttachments::Table)
                    .col(TextChatAttachments::MessageId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_text_chat_attachments_object_key")
                    .table(TextChatAttachments::Table)
                    .col(TextChatAttachments::ObjectKey)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_text_chat_attachments_object_key")
                    .table(TextChatAttachments::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_text_chat_attachments_message")
                    .table(TextChatAttachments::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_text_chat_attachments_room_created")
                    .table(TextChatAttachments::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(TextChatAttachments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TextChatAttachments {
    Table,
    Id,
    ServerId,
    RoomId,
    UploaderUserId,
    MessageId,
    Bucket,
    ObjectKey,
    ContentType,
    ByteSize,
    Width,
    Height,
    Sha256,
    OriginalFilename,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ServerRooms {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum TextMessages {
    Table,
    Id,
}
