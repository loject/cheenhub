//! Google OAuth account linking tables.

use sea_orm_migration::prelude::*;

/// Adds Google OAuth account linking and short-lived OAuth handoff tables.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(ColumnDef::new(Users::PasswordHash).text())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OAuthAccounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OAuthAccounts::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(OAuthAccounts::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(OAuthAccounts::Provider)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthAccounts::ProviderSubject)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthAccounts::Email)
                            .string_len(320)
                            .not_null(),
                    )
                    .col(ColumnDef::new(OAuthAccounts::DisplayName).string_len(255))
                    .col(
                        ColumnDef::new(OAuthAccounts::LinkedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_accounts_user")
                            .from(OAuthAccounts::Table, OAuthAccounts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_accounts_provider_subject")
                    .table(OAuthAccounts::Table)
                    .col(OAuthAccounts::Provider)
                    .col(OAuthAccounts::ProviderSubject)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_accounts_provider_user")
                    .table(OAuthAccounts::Table)
                    .col(OAuthAccounts::Provider)
                    .col(OAuthAccounts::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OAuthStates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OAuthStates::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OAuthStates::StateHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(OAuthStates::Nonce).string_len(96).not_null())
                    .col(
                        ColumnDef::new(OAuthStates::FlowKind)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(ColumnDef::new(OAuthStates::UserId).uuid())
                    .col(
                        ColumnDef::new(OAuthStates::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthStates::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(OAuthStates::ConsumedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_states_user")
                            .from(OAuthStates::Table, OAuthStates::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OAuthRegistrationIntents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OAuthRegistrationIntents::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OAuthRegistrationIntents::Provider)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthRegistrationIntents::ProviderSubject)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthRegistrationIntents::Email)
                            .string_len(320)
                            .not_null(),
                    )
                    .col(ColumnDef::new(OAuthRegistrationIntents::DisplayName).string_len(255))
                    .col(
                        ColumnDef::new(OAuthRegistrationIntents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthRegistrationIntents::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthRegistrationIntents::ConsumedAt)
                            .timestamp_with_time_zone(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OAuthHandoffs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OAuthHandoffs::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OAuthHandoffs::CodeHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(OAuthHandoffs::Kind)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(OAuthHandoffs::UserId).uuid())
                    .col(ColumnDef::new(OAuthHandoffs::RegistrationIntentId).uuid())
                    .col(
                        ColumnDef::new(OAuthHandoffs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthHandoffs::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(OAuthHandoffs::ConsumedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_handoffs_user")
                            .from(OAuthHandoffs::Table, OAuthHandoffs::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_handoffs_registration_intent")
                            .from(OAuthHandoffs::Table, OAuthHandoffs::RegistrationIntentId)
                            .to(
                                OAuthRegistrationIntents::Table,
                                OAuthRegistrationIntents::Id,
                            )
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OAuthHandoffs::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(OAuthRegistrationIntents::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(OAuthStates::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(OAuthAccounts::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(ColumnDef::new(Users::PasswordHash).text().not_null())
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    PasswordHash,
}

#[derive(DeriveIden)]
enum OAuthAccounts {
    #[sea_orm(iden = "oauth_accounts")]
    Table,
    Id,
    UserId,
    Provider,
    ProviderSubject,
    Email,
    DisplayName,
    LinkedAt,
}

#[derive(DeriveIden)]
enum OAuthStates {
    #[sea_orm(iden = "oauth_states")]
    Table,
    Id,
    StateHash,
    Nonce,
    FlowKind,
    UserId,
    CreatedAt,
    ExpiresAt,
    ConsumedAt,
}

#[derive(DeriveIden)]
enum OAuthHandoffs {
    #[sea_orm(iden = "oauth_handoffs")]
    Table,
    Id,
    CodeHash,
    Kind,
    UserId,
    RegistrationIntentId,
    CreatedAt,
    ExpiresAt,
    ConsumedAt,
}

#[derive(DeriveIden)]
enum OAuthRegistrationIntents {
    #[sea_orm(iden = "oauth_registration_intents")]
    Table,
    Id,
    Provider,
    ProviderSubject,
    Email,
    DisplayName,
    CreatedAt,
    ExpiresAt,
    ConsumedAt,
}
