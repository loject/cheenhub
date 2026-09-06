//! Попытки desktop OAuth, связанные внешними ключами с состоянием и handoff.
//! Удаление связанного состояния или handoff также очищает попытку и проверенную личность.

use sea_orm_migration::prelude::*;

/// Создаёт хранение попыток входа через системный браузер.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DesktopOAuthAttempts::Table)
                    .col(
                        ColumnDef::new(DesktopOAuthAttempts::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DesktopOAuthAttempts::OauthStateId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(DesktopOAuthAttempts::SecretHash)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(DesktopOAuthAttempts::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DesktopOAuthAttempts::Status)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DesktopOAuthAttempts::HandoffId)
                            .uuid()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(DesktopOAuthAttempts::ProviderSubject).string())
                    .col(ColumnDef::new(DesktopOAuthAttempts::Email).string())
                    .col(ColumnDef::new(DesktopOAuthAttempts::DisplayName).string())
                    .col(ColumnDef::new(DesktopOAuthAttempts::ErrorMessage).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_desktop_oauth_state")
                            .from(
                                DesktopOAuthAttempts::Table,
                                DesktopOAuthAttempts::OauthStateId,
                            )
                            .to(OAuthStates::Table, OAuthStates::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_desktop_oauth_handoff")
                            .from(DesktopOAuthAttempts::Table, DesktopOAuthAttempts::HandoffId)
                            .to(OAuthHandoffs::Table, OAuthHandoffs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_desktop_oauth_attempts_expiry")
                    .table(DesktopOAuthAttempts::Table)
                    .col(DesktopOAuthAttempts::ExpiresAt)
                    .col(DesktopOAuthAttempts::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DesktopOAuthAttempts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DesktopOAuthAttempts {
    #[sea_orm(iden = "desktop_oauth_attempts")]
    Table,
    Id,
    OauthStateId,
    SecretHash,
    ExpiresAt,
    Status,
    HandoffId,
    ProviderSubject,
    Email,
    DisplayName,
    ErrorMessage,
}

#[derive(DeriveIden)]
enum OAuthStates {
    #[sea_orm(iden = "oauth_states")]
    Table,
    Id,
}

#[derive(DeriveIden)]
enum OAuthHandoffs {
    #[sea_orm(iden = "oauth_handoffs")]
    Table,
    Id,
}
