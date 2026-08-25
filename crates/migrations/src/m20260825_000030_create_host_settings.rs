//! Создаёт владельцев хоста и настройки исходящей почты.

use sea_orm_migration::prelude::*;

const CREATE_INITIAL_HOST_OWNER_FUNCTION: &str = r#"
CREATE OR REPLACE FUNCTION cheenhub_assign_initial_host_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM host_owners) THEN
        RETURN NEW;
    END IF;
    LOCK TABLE host_owners IN SHARE ROW EXCLUSIVE MODE;
    INSERT INTO host_owners (user_id, granted_at, granted_by_user_id)
    SELECT NEW.id, NEW.registered_at, NULL
    WHERE NOT EXISTS (SELECT 1 FROM host_owners);
    RETURN NEW;
END;
$$
"#;

const DROP_INITIAL_HOST_OWNER_TRIGGER: &str =
    "DROP TRIGGER IF EXISTS cheenhub_assign_initial_host_owner ON users";

const CREATE_INITIAL_HOST_OWNER_TRIGGER: &str = r#"
CREATE TRIGGER cheenhub_assign_initial_host_owner
AFTER INSERT ON users
FOR EACH ROW
EXECUTE FUNCTION cheenhub_assign_initial_host_owner()
"#;

const DROP_INITIAL_HOST_OWNER_FUNCTION: &str =
    "DROP FUNCTION IF EXISTS cheenhub_assign_initial_host_owner()";

/// Миграция глобальных настроек конкретной установки CheenHub.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HostOwners::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HostOwners::UserId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HostOwners::GrantedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HostOwners::GrantedByUserId).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_host_owners_user")
                            .from(HostOwners::Table, HostOwners::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_host_owners_granted_by_user")
                            .from(HostOwners::Table, HostOwners::GrantedByUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        let first_user = Query::select()
            .column(Users::Id)
            .expr(Expr::current_timestamp())
            .from(Users::Table)
            .order_by(Users::RegisteredAt, Order::Asc)
            .order_by(Users::Id, Order::Asc)
            .limit(1)
            .to_owned();
        let mut bootstrap_owner = Query::insert();
        bootstrap_owner
            .into_table(HostOwners::Table)
            .columns([HostOwners::UserId, HostOwners::GrantedAt])
            .select_from(first_user)
            .map_err(|error| DbErr::Migration(error.to_string()))?;
        manager.exec_stmt(bootstrap_owner).await?;

        manager
            .create_table(
                Table::create()
                    .table(HostEmailSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HostEmailSettings::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HostEmailSettings::Transport)
                            .string_len(16)
                            .not_null()
                            .default("smtp"),
                    )
                    .col(
                        ColumnDef::new(HostEmailSettings::EmailSendTimeoutSeconds)
                            .integer()
                            .not_null()
                            .default(10),
                    )
                    .col(ColumnDef::new(HostEmailSettings::SmtpHost).string_len(255))
                    .col(ColumnDef::new(HostEmailSettings::SmtpPort).integer())
                    .col(ColumnDef::new(HostEmailSettings::SmtpUsername).string_len(320))
                    .col(ColumnDef::new(HostEmailSettings::SmtpPassword).text())
                    .col(ColumnDef::new(HostEmailSettings::SmtpFromEmail).string_len(320))
                    .col(ColumnDef::new(HostEmailSettings::GmailClientId).text())
                    .col(ColumnDef::new(HostEmailSettings::GmailClientSecret).text())
                    .col(ColumnDef::new(HostEmailSettings::GmailRefreshToken).text())
                    .col(ColumnDef::new(HostEmailSettings::GmailFromEmail).string_len(320))
                    .col(
                        ColumnDef::new(HostEmailSettings::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(HostEmailSettings::UpdatedByUserId).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_host_email_settings_updated_by_user")
                            .from(HostEmailSettings::Table, HostEmailSettings::UpdatedByUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .check(
                        Expr::col(HostEmailSettings::Id).eq("00000000-0000-0000-0000-000000000000"),
                    )
                    .check(Expr::col(HostEmailSettings::Transport).is_in(["smtp", "gmail_api"]))
                    .check(Expr::col(HostEmailSettings::EmailSendTimeoutSeconds).between(1, 300))
                    .check(
                        Expr::col(HostEmailSettings::SmtpPort)
                            .is_null()
                            .or(Expr::col(HostEmailSettings::SmtpPort).between(1, 65_535)),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(HostGmailOAuthStates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HostGmailOAuthStates::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HostGmailOAuthStates::StateHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(HostGmailOAuthStates::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HostGmailOAuthStates::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HostGmailOAuthStates::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HostGmailOAuthStates::ConsumedAt).timestamp_with_time_zone(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_host_gmail_oauth_states_user")
                            .from(HostGmailOAuthStates::Table, HostGmailOAuthStates::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Триггер PostgreSQL нужен для атомарного выбора ровно одного владельца при
        // одновременной первой регистрации; migration DSL не выражает триггеры и table lock.
        manager
            .get_connection()
            .execute_unprepared(CREATE_INITIAL_HOST_OWNER_FUNCTION)
            .await?;
        manager
            .get_connection()
            .execute_unprepared(DROP_INITIAL_HOST_OWNER_TRIGGER)
            .await?;
        manager
            .get_connection()
            .execute_unprepared(CREATE_INITIAL_HOST_OWNER_TRIGGER)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(DROP_INITIAL_HOST_OWNER_TRIGGER)
            .await?;
        manager
            .get_connection()
            .execute_unprepared(DROP_INITIAL_HOST_OWNER_FUNCTION)
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(HostGmailOAuthStates::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(LegacyHostGmailOAuthStates::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(HostEmailSettings::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(HostOwners::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    RegisteredAt,
}

#[derive(DeriveIden)]
enum HostOwners {
    Table,
    UserId,
    GrantedAt,
    GrantedByUserId,
}

#[derive(DeriveIden)]
enum HostEmailSettings {
    Table,
    Id,
    Transport,
    EmailSendTimeoutSeconds,
    SmtpHost,
    SmtpPort,
    SmtpUsername,
    SmtpPassword,
    SmtpFromEmail,
    GmailClientId,
    GmailClientSecret,
    GmailRefreshToken,
    GmailFromEmail,
    UpdatedAt,
    UpdatedByUserId,
}

#[derive(DeriveIden)]
enum HostGmailOAuthStates {
    #[sea_orm(iden = "host_gmail_oauth_states")]
    Table,
    Id,
    StateHash,
    UserId,
    CreatedAt,
    ExpiresAt,
    ConsumedAt,
}

#[derive(DeriveIden)]
enum LegacyHostGmailOAuthStates {
    #[sea_orm(iden = "host_gmail_o_auth_states")]
    Table,
}
