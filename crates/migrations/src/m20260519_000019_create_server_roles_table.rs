//! Server role tables.

use sea_orm_migration::prelude::*;

/// Creates the server_roles table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerRoles::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerRoles::ServerId).uuid().not_null())
                    .col(ColumnDef::new(ServerRoles::Name).string_len(32).not_null())
                    .col(ColumnDef::new(ServerRoles::Color).string_len(7).not_null())
                    .col(ColumnDef::new(ServerRoles::Kind).string_len(16).not_null())
                    .col(ColumnDef::new(ServerRoles::Position).integer().not_null())
                    .col(
                        ColumnDef::new(ServerRoles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerRoles::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_roles_server")
                            .from(ServerRoles::Table, ServerRoles::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_roles_server_position")
                    .table(ServerRoles::Table)
                    .col(ServerRoles::ServerId)
                    .col(ServerRoles::Position)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ServerRolePermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerRolePermissions::RoleId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerRolePermissions::Permission)
                            .string_len(48)
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ServerRolePermissions::RoleId)
                            .col(ServerRolePermissions::Permission),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_role_permissions_role")
                            .from(ServerRolePermissions::Table, ServerRolePermissions::RoleId)
                            .to(ServerRoles::Table, ServerRoles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_role_permissions_permission")
                    .table(ServerRolePermissions::Table)
                    .col(ServerRolePermissions::Permission)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_role_permissions_permission")
                    .table(ServerRolePermissions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerRolePermissions::Table).to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_roles_server_position")
                    .table(ServerRoles::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerRoles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerRoles {
    Table,
    Id,
    ServerId,
    Name,
    Color,
    Kind,
    Position,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ServerRolePermissions {
    Table,
    RoleId,
    Permission,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}
