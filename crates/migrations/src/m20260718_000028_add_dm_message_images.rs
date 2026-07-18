//! Добавляет изображения к личным сообщениям.

use sea_orm_migration::prelude::*;

/// Миграция ссылки личного сообщения на таблицу изображений.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DmMessages::Table)
                    .add_column(ColumnDef::new(DmMessages::ImageId).uuid().null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_dm_messages_image_id")
                    .from(DmMessages::Table, DmMessages::ImageId)
                    .to(Images::Table, Images::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_dm_messages_image_id_unique")
                    .table(DmMessages::Table)
                    .col(DmMessages::ImageId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_dm_messages_image_id_unique")
                    .table(DmMessages::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_dm_messages_image_id")
                    .table(DmMessages::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DmMessages::Table)
                    .drop_column(DmMessages::ImageId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum DmMessages {
    Table,
    ImageId,
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
}
