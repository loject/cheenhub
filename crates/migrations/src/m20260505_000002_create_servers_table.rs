//! Server tables.

use sea_orm_migration::prelude::*;

/// Creates the servers table.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Servers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Servers::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Servers::OwnerUserId).uuid().not_null())
                    .col(ColumnDef::new(Servers::Name).string_len(48).not_null())
                    .col(
                        ColumnDef::new(Servers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Servers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_servers_owner_user")
                            .from(Servers::Table, Servers::OwnerUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_servers_owner_user_id")
                    .table(Servers::Table)
                    .col(Servers::OwnerUserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_servers_owner_user_id")
                    .table(Servers::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Servers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
    OwnerUserId,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
