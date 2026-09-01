//! Adds the per-server target voice audio bitrate.

use sea_orm_migration::prelude::*;

/// Adds the server target Opus audio bitrate column.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Servers::Table)
                    .add_column(
                        ColumnDef::new(Servers::AudioBitrateBps)
                            .integer()
                            .not_null()
                            .default(32_000),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Servers::Table)
                    .drop_column(Servers::AudioBitrateBps)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    AudioBitrateBps,
}
