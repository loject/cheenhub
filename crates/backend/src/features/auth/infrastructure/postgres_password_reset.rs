//! Вспомогательные функции хранения сброса пароля для Postgres.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait, sea_query::LockType,
};
use uuid::Uuid;

use crate::features::auth::domain::PasswordResetToken;
use crate::features::auth::infrastructure::entities::{password_reset_tokens, sessions, users};

pub(super) async fn revoke_user_sessions(
    database: &DatabaseConnection,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    sessions::Entity::update_many()
        .col_expr(
            sessions::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(sessions::Column::UserId.eq(*user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .exec(database)
        .await?;

    Ok(())
}

pub(super) async fn insert_password_reset_token(
    database: &DatabaseConnection,
    user_id: &Uuid,
    token_hash: String,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let transaction = database.begin().await?;
    users::Entity::find_by_id(*user_id)
        .lock(LockType::Update)
        .one(&transaction)
        .await?
        .ok_or_else(|| anyhow::anyhow!("password reset user is missing"))?;
    password_reset_tokens::Entity::update_many()
        .col_expr(
            password_reset_tokens::Column::ConsumedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(password_reset_tokens::Column::UserId.eq(*user_id))
        .filter(password_reset_tokens::Column::ConsumedAt.is_null())
        .exec(&transaction)
        .await?;
    password_reset_tokens::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(*user_id),
        token_hash: Set(token_hash),
        created_at: Set(now),
        expires_at: Set(expires_at),
        consumed_at: Set(None),
    }
    .insert(&transaction)
    .await?;
    transaction.commit().await?;

    Ok(())
}

pub(super) async fn find_active_password_reset_token(
    database: &DatabaseConnection,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<PasswordResetToken>> {
    Ok(password_reset_tokens::Entity::find()
        .filter(password_reset_tokens::Column::TokenHash.eq(token_hash))
        .filter(password_reset_tokens::Column::ConsumedAt.is_null())
        .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
        .map(Into::into))
}

pub(super) async fn complete_password_reset(
    database: &DatabaseConnection,
    token_hash: &str,
    password_hash: String,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<PasswordResetToken>> {
    let transaction = database.begin().await?;
    let Some(token) = password_reset_tokens::Entity::find()
        .filter(password_reset_tokens::Column::TokenHash.eq(token_hash))
        .filter(password_reset_tokens::Column::ConsumedAt.is_null())
        .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
        .one(&transaction)
        .await?
    else {
        transaction.rollback().await?;
        return Ok(None);
    };
    let user_id = token.user_id;
    users::Entity::find_by_id(user_id)
        .lock(LockType::Update)
        .one(&transaction)
        .await?
        .ok_or_else(|| anyhow::anyhow!("password reset user is missing"))?;
    let consumed = password_reset_tokens::Entity::update_many()
        .col_expr(
            password_reset_tokens::Column::ConsumedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(password_reset_tokens::Column::Id.eq(token.id))
        .filter(password_reset_tokens::Column::ConsumedAt.is_null())
        .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
        .exec(&transaction)
        .await?;
    if consumed.rows_affected != 1 {
        transaction.rollback().await?;
        return Ok(None);
    }
    users::Entity::update_many()
        .col_expr(
            users::Column::PasswordHash,
            sea_orm::sea_query::Expr::value(Some(password_hash)),
        )
        .col_expr(
            users::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(users::Column::Id.eq(user_id))
        .exec(&transaction)
        .await?;
    sessions::Entity::update_many()
        .col_expr(
            sessions::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(sessions::Column::UserId.eq(user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .exec(&transaction)
        .await?;
    password_reset_tokens::Entity::update_many()
        .col_expr(
            password_reset_tokens::Column::ConsumedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(password_reset_tokens::Column::UserId.eq(user_id))
        .filter(password_reset_tokens::Column::ConsumedAt.is_null())
        .exec(&transaction)
        .await?;
    transaction.commit().await?;

    Ok(Some(token.into()))
}
