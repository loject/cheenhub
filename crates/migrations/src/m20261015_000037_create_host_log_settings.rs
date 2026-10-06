//! Создаёт настройку минимального уровня журналирования хоста.
//!
//! Уровень хранится одной строкой-singleton и применяется ко всему процессу,
//! поэтому отдельная таблица не смешивается с настройками исходящей почты.
//! Пустой `min_level` означает «использовать фильтр, заданный при запуске»:
//! так владелец может вернуть сервер к конфигурации окружения без миграции.

use sea_orm_migration::prelude::*;

/// Миграция настройки журналирования конкретной установки CheenHub.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HostLogSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HostLogSettings::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(HostLogSettings::MinLevel).string())
                    .col(
                        ColumnDef::new(HostLogSettings::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HostLogSettings::UpdatedByUserId).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_host_log_settings_user")
                            .from(HostLogSettings::Table, HostLogSettings::UpdatedByUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(HostLogSettings::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum HostLogSettings {
    Table,
    Id,
    MinLevel,
    UpdatedAt,
    UpdatedByUserId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
