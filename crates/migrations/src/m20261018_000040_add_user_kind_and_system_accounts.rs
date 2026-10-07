//! Добавляет типы аккаунтов и системную запись для каждого сервера.

use chrono::{DateTime, Utc};
use sea_orm_migration::prelude::*;
use uuid::Uuid;

const INSERT_BATCH_SIZE: usize = 500;

/// Добавляет системные аккаунты серверов и отделяет их ограничения от обычных пользователей.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_foreign_key(Alias::new("users_nickname_key"))
                    .drop_foreign_key(Alias::new("users_email_normalized_key"))
                    .modify_column(ColumnDef::new(Users::Nickname).string_len(48).not_null())
                    .modify_column(ColumnDef::new(Users::Email).string_len(320).null())
                    .modify_column(
                        ColumnDef::new(Users::EmailNormalized)
                            .string_len(320)
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Users::Kind)
                            .string_len(16)
                            .default("user")
                            .not_null()
                            .check(
                                Expr::col(Users::Kind)
                                    .is_in(["user", "system", "bot"])
                                    .and(
                                        Expr::col(Users::Kind)
                                            .eq("user")
                                            .and(Expr::col(Users::Email).is_not_null())
                                            .and(Expr::col(Users::EmailNormalized).is_not_null()),
                                    )
                                    .or(Expr::col(Users::Kind)
                                        .ne("user")
                                        .and(Expr::col(Users::Email).is_null())
                                        .and(Expr::col(Users::EmailNormalized).is_null())
                                        .and(Expr::col(Users::PasswordHash).is_null())),
                            ),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_users_user_nickname_unique")
                    .table(Users::Table)
                    .col(Users::Nickname)
                    .unique()
                    .and_where(Expr::col(Users::Kind).eq("user"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_users_user_email_normalized_unique")
                    .table(Users::Table)
                    .col(Users::EmailNormalized)
                    .unique()
                    .and_where(Expr::col(Users::Kind).eq("user"))
                    .to_owned(),
            )
            .await?;

        let servers = manager
            .get_connection()
            .query_all(&server_accounts_to_insert())
            .await?
            .into_iter()
            .map(|row| {
                Ok(SystemAccount {
                    id: row.try_get("", "id")?,
                    nickname: row.try_get("", "name")?,
                    avatar_image_id: row.try_get("", "avatar_image_id")?,
                    created_at: row.try_get("", "created_at")?,
                    updated_at: row.try_get("", "updated_at")?,
                })
            })
            .collect::<Result<Vec<_>, DbErr>>()?;

        for batch in servers.chunks(INSERT_BATCH_SIZE) {
            manager.execute(insert_system_accounts(batch)).await?;
        }

        tracing::info!(
            system_accounts = servers.len(),
            "созданы системные аккаунты для существующих серверов"
        );

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.execute(delete_non_user_accounts()).await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_user_nickname_unique")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_user_email_normalized_unique")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Kind)
                    .modify_column(ColumnDef::new(Users::Nickname).string_len(32).not_null())
                    .modify_column(ColumnDef::new(Users::Email).string_len(320).not_null())
                    .modify_column(
                        ColumnDef::new(Users::EmailNormalized)
                            .string_len(320)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("users_nickname_key")
                    .table(Users::Table)
                    .col(Users::Nickname)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("users_email_normalized_key")
                    .table(Users::Table)
                    .col(Users::EmailNormalized)
                    .unique()
                    .to_owned(),
            )
            .await
    }
}

struct SystemAccount {
    id: Uuid,
    nickname: String,
    avatar_image_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn server_accounts_to_insert() -> SelectStatement {
    Query::select()
        .column(Servers::Id)
        .column(Servers::Name)
        .column(Servers::AvatarImageId)
        .column(Servers::CreatedAt)
        .column(Servers::UpdatedAt)
        .from(Servers::Table)
        .to_owned()
}

fn insert_system_accounts(accounts: &[SystemAccount]) -> InsertStatement {
    let mut insert = Query::insert();
    insert.into_table(Users::Table).columns([
        Users::Id,
        Users::Nickname,
        Users::Email,
        Users::EmailNormalized,
        Users::PasswordHash,
        Users::AvatarImageId,
        Users::RegisteredAt,
        Users::NicknameUpdatedAt,
        Users::UpdatedAt,
        Users::Kind,
    ]);

    for account in accounts {
        insert.values_panic([
            account.id.into(),
            account.nickname.clone().into(),
            Option::<String>::None.into(),
            Option::<String>::None.into(),
            Option::<String>::None.into(),
            account.avatar_image_id.into(),
            account.created_at.into(),
            account.updated_at.into(),
            account.updated_at.into(),
            "system".into(),
        ]);
    }

    insert.take()
}

fn delete_non_user_accounts() -> DeleteStatement {
    Query::delete()
        .from_table(Users::Table)
        .cond_where(Expr::col(Users::Kind).ne("user"))
        .to_owned()
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Nickname,
    Email,
    EmailNormalized,
    PasswordHash,
    AvatarImageId,
    RegisteredAt,
    NicknameUpdatedAt,
    UpdatedAt,
    Kind,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
    Name,
    AvatarImageId,
    CreatedAt,
    UpdatedAt,
}
