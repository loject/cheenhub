//! Server invite tables.

use sea_orm_migration::prelude::*;

/// Creates the server_invites table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerInvites::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerInvites::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerInvites::ServerId).uuid().not_null())
                    .col(
                        ColumnDef::new(ServerInvites::CreatorUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ServerInvites::MaxUses).integer())
                    .col(ColumnDef::new(ServerInvites::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ServerInvites::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_invites_server")
                            .from(ServerInvites::Table, ServerInvites::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_invites_creator_user")
                            .from(ServerInvites::Table, ServerInvites::CreatorUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_invites_server_id")
                    .table(ServerInvites::Table)
                    .col(ServerInvites::ServerId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_invites_server_id")
                    .table(ServerInvites::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ServerInvites::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ServerInvites {
    Table,
    Id,
    ServerId,
    CreatorUserId,
    MaxUses,
    ExpiresAt,
    CreatedAt,
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
