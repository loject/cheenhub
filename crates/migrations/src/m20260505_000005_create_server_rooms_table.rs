//! Server room tables.

use sea_orm_migration::prelude::*;

/// Creates the server_rooms table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerRooms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerRooms::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerRooms::ServerId).uuid().not_null())
                    .col(ColumnDef::new(ServerRooms::Name).string_len(48).not_null())
                    .col(ColumnDef::new(ServerRooms::Kind).string_len(24).not_null())
                    .col(ColumnDef::new(ServerRooms::Position).integer().not_null())
                    .col(
                        ColumnDef::new(ServerRooms::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerRooms::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_rooms_server")
                            .from(ServerRooms::Table, ServerRooms::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_rooms_server_position")
                    .table(ServerRooms::Table)
                    .col(ServerRooms::ServerId)
                    .col(ServerRooms::Position)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_rooms_server_position")
                    .table(ServerRooms::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerRooms::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerRooms {
    Table,
    Id,
    ServerId,
    Name,
    Kind,
    Position,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}
