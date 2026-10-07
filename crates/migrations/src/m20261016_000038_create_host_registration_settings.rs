//! Создаёт настройки способов регистрации пользователей хоста.

use sea_orm_migration::prelude::*;

/// Миграция доступности создания аккаунтов на хосте.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HostRegistrationSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HostRegistrationSettings::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HostRegistrationSettings::RegistrationEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(HostRegistrationSettings::EmailPasswordRegistrationEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(HostRegistrationSettings::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HostRegistrationSettings::UpdatedByUserId).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_host_registration_settings_user")
                            .from(
                                HostRegistrationSettings::Table,
                                HostRegistrationSettings::UpdatedByUserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .check(
                        Expr::col(HostRegistrationSettings::Id)
                            .eq("00000000-0000-0000-0000-000000000000"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(HostRegistrationSettings::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum HostRegistrationSettings {
    Table,
    Id,
    RegistrationEnabled,
    EmailPasswordRegistrationEnabled,
    UpdatedAt,
    UpdatedByUserId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
