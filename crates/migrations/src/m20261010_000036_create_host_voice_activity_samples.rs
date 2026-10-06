//! Создаёт историю измерений активности голосового чата хоста.
//!
//! Таблица хранит оба показателя активности: число голосовых подключений и число
//! активных видеоисточников. Значения по умолчанию не задаются: ноль подключений
//! и ноль видеоисточников — полноценные измерения, которые нужно отличать от
//! отсутствующей строки.

use sea_orm_migration::prelude::*;

/// Создаёт таблицу снимков активности голосового чата.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HostVoiceActivitySamples::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HostVoiceActivitySamples::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HostVoiceActivitySamples::SampledAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HostVoiceActivitySamples::VoiceConnections)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HostVoiceActivitySamples::VideoSources)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Индекс по времени обслуживает запрос последних 24 часов и удаление старых измерений.
        manager
            .create_index(
                Index::create()
                    .name("idx_host_voice_activity_samples_sampled_at")
                    .table(HostVoiceActivitySamples::Table)
                    .col(HostVoiceActivitySamples::SampledAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(HostVoiceActivitySamples::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum HostVoiceActivitySamples {
    Table,
    Id,
    SampledAt,
    VoiceConnections,
    VideoSources,
}
