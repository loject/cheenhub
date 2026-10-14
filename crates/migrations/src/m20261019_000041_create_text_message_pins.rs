//! Закрепления сообщений в комнатах сервера.

use sea_orm_migration::prelude::*;

/// Создаёт таблицу закреплённых сообщений.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Pins::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Pins::MessageId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Pins::ServerId).uuid().not_null())
                    .col(ColumnDef::new(Pins::RoomId).uuid().not_null())
                    .col(ColumnDef::new(Pins::PinnedByUserId).uuid().not_null())
                    .col(
                        ColumnDef::new(Pins::PinnedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_message_pins_message")
                            .from(Pins::Table, Pins::MessageId)
                            .to(TextMessages::Table, TextMessages::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_message_pins_server")
                            .from(Pins::Table, Pins::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_text_message_pins_room")
                            .from(Pins::Table, Pins::RoomId)
                            .to(ServerRooms::Table, ServerRooms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_text_message_pins_room_created")
                    .table(Pins::Table)
                    .col(Pins::RoomId)
                    .col(Pins::PinnedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_text_message_pins_room_created")
                    .table(Pins::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Pins::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Pins {
    Table,
    MessageId,
    ServerId,
    RoomId,
    PinnedByUserId,
    PinnedAt,
}

#[derive(DeriveIden)]
enum TextMessages {
    Table,
    Id,
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
