//! Server member exclusion table.

use sea_orm_migration::prelude::*;

/// Creates the server_member_exclusions table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerMemberExclusions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerMemberExclusions::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ServerMemberExclusions::ServerId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerMemberExclusions::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerMemberExclusions::InitiatorUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerMemberExclusions::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerMemberExclusions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_member_exclusions_server")
                            .from(
                                ServerMemberExclusions::Table,
                                ServerMemberExclusions::ServerId,
                            )
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_member_exclusions_user")
                            .from(
                                ServerMemberExclusions::Table,
                                ServerMemberExclusions::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_server_member_exclusions_initiator")
                            .from(
                                ServerMemberExclusions::Table,
                                ServerMemberExclusions::InitiatorUserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_server_member_exclusions_server_user_expires_at")
                    .table(ServerMemberExclusions::Table)
                    .col(ServerMemberExclusions::ServerId)
                    .col(ServerMemberExclusions::UserId)
                    .col(ServerMemberExclusions::ExpiresAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_server_member_exclusions_server_user_expires_at")
                    .table(ServerMemberExclusions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(ServerMemberExclusions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ServerMemberExclusions {
    Table,
    Id,
    ServerId,
    UserId,
    InitiatorUserId,
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
