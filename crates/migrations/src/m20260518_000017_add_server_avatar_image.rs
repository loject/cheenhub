//! Links servers to uploaded avatar images.

use sea_orm_migration::prelude::*;

/// Adds a nullable server avatar image reference.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut avatar_foreign_key = TableForeignKey::new();
        avatar_foreign_key
            .name("fk_servers_avatar_image")
            .from_tbl(Servers::Table)
            .from_col(Servers::AvatarImageId)
            .to_tbl(Images::Table)
            .to_col(Images::Id)
            .on_delete(ForeignKeyAction::SetNull);

        manager
            .alter_table(
                Table::alter()
                    .table(Servers::Table)
                    .add_column(ColumnDef::new(Servers::AvatarImageId).uuid())
                    .add_foreign_key(&avatar_foreign_key)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Servers::Table)
                    .drop_foreign_key(Alias::new("fk_servers_avatar_image"))
                    .drop_column(Servers::AvatarImageId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    AvatarImageId,
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
}
