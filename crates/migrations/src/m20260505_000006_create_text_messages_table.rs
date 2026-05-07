//! Text message table.

use sea_orm_migration::prelude::*;

/// Creates the text_messages table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TextMessages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TextMessages::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TextMessages::ServerId).uuid().not_null())
                    .col(ColumnDef::new(TextMessages::RoomId).uuid().not_null())
                    .col(ColumnDef::new(TextMessages::AuthorUserId).uuid().not_null())
                    .col(
                        ColumnDef::new(TextMessages::AuthorNickname)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextMessages::Body)
                            .string_len(2000)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TextMessages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_messages_server")
                            .from(TextMessages::Table, TextMessages::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_messages_room")
                            .from(TextMessages::Table, TextMessages::RoomId)
                            .to(ServerRooms::Table, ServerRooms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_messages_author")
                            .from(TextMessages::Table, TextMessages::AuthorUserId)
                            .to(Users::Table, Users::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_text_messages_room_created")
                    .table(TextMessages::Table)
                    .col(TextMessages::RoomId)
                    .col(TextMessages::CreatedAt)
                    .col(TextMessages::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_text_messages_room_created")
                    .table(TextMessages::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(TextMessages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TextMessages {
    Table,
    Id,
    ServerId,
    RoomId,
    AuthorUserId,
    AuthorNickname,
    Body,
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
