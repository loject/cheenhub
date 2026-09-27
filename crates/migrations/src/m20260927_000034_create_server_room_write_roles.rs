//! Server room write access tables.

use sea_orm_migration::prelude::*;

/// Creates the server_rooms write access mode column and the server_room_write_roles table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ServerRooms::Table)
                    .add_column(
                        ColumnDef::new(ServerRooms::WriteAccessMode)
                            .string_len(24)
                            .not_null()
                            .default("all_members"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ServerRoomWriteRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerRoomWriteRoles::RoomId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerRoomWriteRoles::RoleId)
                            .uuid()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ServerRoomWriteRoles::RoomId)
                            .col(ServerRoomWriteRoles::RoleId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_room_write_roles_room")
                            .from(ServerRoomWriteRoles::Table, ServerRoomWriteRoles::RoomId)
                            .to(ServerRooms::Table, ServerRooms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_room_write_roles_role")
                            .from(ServerRoomWriteRoles::Table, ServerRoomWriteRoles::RoleId)
                            .to(ServerRoles::Table, ServerRoles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_room_write_roles_role")
                    .table(ServerRoomWriteRoles::Table)
                    .col(ServerRoomWriteRoles::RoleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_room_write_roles_role")
                    .table(ServerRoomWriteRoles::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerRoomWriteRoles::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ServerRooms::Table)
                    .drop_column(ServerRooms::WriteAccessMode)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ServerRoomWriteRoles {
    Table,
    RoomId,
    RoleId,
}

#[derive(DeriveIden)]
enum ServerRooms {
    Table,
    Id,
    WriteAccessMode,
}

#[derive(DeriveIden)]
enum ServerRoles {
    Table,
    Id,
}
