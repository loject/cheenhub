//! Postgres password reset storage helpers.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::features::auth::domain::PasswordResetToken;
use crate::features::auth::infrastructure::entities::{password_reset_tokens, sessions, users};

pub(super) async fn update_user_password_hash(
    database: &DatabaseConnection,
    user_id: &Uuid,
    password_hash: String,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    if let Some(user) = users::Entity::find_by_id(*user_id).one(database).await? {
        let mut user = user.into_active_model();
        user.password_hash = Set(Some(password_hash));
        user.updated_at = Set(now);
        user.update(database).await?;
    }

    Ok(())
}

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
    password_reset_tokens::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(*user_id),
        token_hash: Set(token_hash),
        created_at: Set(now),
        expires_at: Set(expires_at),
        consumed_at: Set(None),
    }
    .insert(database)
    .await?;

    Ok(())
}

pub(super) async fn consume_password_reset_token(
    database: &DatabaseConnection,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<PasswordResetToken>> {
    let Some(token) = password_reset_tokens::Entity::find()
        .filter(password_reset_tokens::Column::TokenHash.eq(token_hash))
        .filter(password_reset_tokens::Column::ConsumedAt.is_null())
        .filter(password_reset_tokens::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
    else {
        return Ok(None);
    };

    let result = token.clone().into();
    let mut active = token.into_active_model();
    active.consumed_at = Set(Some(now));
    active.update(database).await?;

    Ok(Some(result))
}
