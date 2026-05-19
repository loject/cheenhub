//! Server member role assignment table.

use sea_orm_migration::prelude::*;

/// Creates the server_member_roles table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerMemberRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerMemberRoles::ServerId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ServerMemberRoles::UserId).uuid().not_null())
                    .col(ColumnDef::new(ServerMemberRoles::RoleId).uuid().not_null())
                    .col(
                        ColumnDef::new(ServerMemberRoles::GrantedByUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerMemberRoles::AssignedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ServerMemberRoles::ServerId)
                            .col(ServerMemberRoles::UserId)
                            .col(ServerMemberRoles::RoleId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_member_roles_server")
                            .from(ServerMemberRoles::Table, ServerMemberRoles::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_member_roles_role")
                            .from(ServerMemberRoles::Table, ServerMemberRoles::RoleId)
                            .to(ServerRoles::Table, ServerRoles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_member_roles_server_user")
                    .table(ServerMemberRoles::Table)
                    .col(ServerMemberRoles::ServerId)
                    .col(ServerMemberRoles::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_member_roles_server_user")
                    .table(ServerMemberRoles::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerMemberRoles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerMemberRoles {
    Table,
    ServerId,
    UserId,
    RoleId,
    GrantedByUserId,
    AssignedAt,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ServerRoles {
    Table,
    Id,
}
