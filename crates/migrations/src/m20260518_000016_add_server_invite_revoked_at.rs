//! Adds revocation timestamp to server invites.

use sea_orm_migration::prelude::*;

/// Adds server_invites.revoked_at.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ServerInvites::Table)
                    .add_column(ColumnDef::new(ServerInvites::RevokedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ServerInvites::Table)
                    .drop_column(ServerInvites::RevokedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ServerInvites {
    Table,
    RevokedAt,
}
