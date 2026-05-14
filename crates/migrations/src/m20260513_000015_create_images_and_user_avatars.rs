//! Adds database-backed image rows and user avatar references.

use sea_orm_migration::prelude::*;

/// Creates the images table and links users to their avatar image.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut avatar_foreign_key = TableForeignKey::new();
        avatar_foreign_key
            .name("fk_users_avatar_image")
            .from_tbl(Users::Table)
            .from_col(Users::AvatarImageId)
            .to_tbl(Images::Table)
            .to_col(Images::Id)
            .on_delete(ForeignKeyAction::SetNull);

        manager
            .create_table(
                Table::create()
                    .table(Images::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Images::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Images::OwnerUserId).uuid().not_null())
                    .col(ColumnDef::new(Images::Kind).string_len(64).not_null())
                    .col(
                        ColumnDef::new(Images::ContentType)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Images::Width).integer().not_null())
                    .col(ColumnDef::new(Images::Height).integer().not_null())
                    .col(ColumnDef::new(Images::ByteSize).big_integer().not_null())
                    .col(ColumnDef::new(Images::Sha256).string_len(64).not_null())
                    .col(
                        ColumnDef::new(Images::StorageBackend)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Images::StorageKey).text())
                    .col(ColumnDef::new(Images::Data).binary())
                    .col(
                        ColumnDef::new(Images::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Images::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_images_owner_user")
                            .from(Images::Table, Images::OwnerUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_images_owner_kind_created")
                    .table(Images::Table)
                    .col(Images::OwnerUserId)
                    .col(Images::Kind)
                    .col(Images::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::AvatarImageId).uuid())
                    .add_foreign_key(&avatar_foreign_key)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_foreign_key(Alias::new("fk_users_avatar_image"))
                    .drop_column(Users::AvatarImageId)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_images_owner_kind_created")
                    .table(Images::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Images::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
    OwnerUserId,
    Kind,
    ContentType,
    Width,
    Height,
    ByteSize,
    Sha256,
    StorageBackend,
    StorageKey,
    Data,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    AvatarImageId,
}
