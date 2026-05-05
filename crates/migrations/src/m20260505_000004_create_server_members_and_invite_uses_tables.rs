//! Server membership and invite usage tables.

use sea_orm_migration::prelude::*;

/// Creates server_members and server_invite_uses tables.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerMembers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerMembers::ServerId).uuid().not_null())
                    .col(ColumnDef::new(ServerMembers::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(ServerMembers::JoinedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ServerMembers::LeftAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_members_server")
                            .from(ServerMembers::Table, ServerMembers::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_members_user")
                            .from(ServerMembers::Table, ServerMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_members_server_user_left_at")
                    .table(ServerMembers::Table)
                    .col(ServerMembers::ServerId)
                    .col(ServerMembers::UserId)
                    .col(ServerMembers::LeftAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ServerInviteUses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerInviteUses::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerInviteUses::InviteId).uuid().not_null())
                    .col(ColumnDef::new(ServerInviteUses::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(ServerInviteUses::UsedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_invite_uses_invite")
                            .from(ServerInviteUses::Table, ServerInviteUses::InviteId)
                            .to(ServerInvites::Table, ServerInvites::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_invite_uses_user")
                            .from(ServerInviteUses::Table, ServerInviteUses::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_invite_uses_invite_id")
                    .table(ServerInviteUses::Table)
                    .col(ServerInviteUses::InviteId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_invite_uses_invite_id")
                    .table(ServerInviteUses::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerInviteUses::Table).to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_members_server_user_left_at")
                    .table(ServerMembers::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerMembers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerMembers {
    Table,
    Id,
    ServerId,
    UserId,
    JoinedAt,
    LeftAt,
}

#[derive(DeriveIden)]
enum ServerInviteUses {
    Table,
    Id,
    InviteId,
    UserId,
    UsedAt,
}

#[derive(DeriveIden)]
enum ServerInvites {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
