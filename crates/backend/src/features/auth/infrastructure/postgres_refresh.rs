//! Postgres refresh token storage helpers.

use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::features::auth::domain::RefreshSession;
use crate::features::auth::infrastructure::entities::{refresh_tokens, sessions, users};

pub(super) async fn find_active_refresh(
    database: &DatabaseConnection,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<RefreshSession>> {
    let Some(refresh_token) = refresh_tokens::Entity::find()
        .filter(refresh_tokens::Column::TokenHash.eq(token_hash))
        .filter(refresh_tokens::Column::RevokedAt.is_null())
        .filter(refresh_tokens::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    let Some(session) = sessions::Entity::find_by_id(refresh_token.session_id)
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    let Some(user) = users::Entity::find_by_id(session.user_id)
        .one(database)
        .await?
    else {
        return Ok(None);
    };

    Ok(Some(RefreshSession {
        refresh_token_id: refresh_token.id,
        session_id: session.id,
        user: user.into(),
    }))
}

pub(super) async fn session_is_active(
    database: &DatabaseConnection,
    session_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    Ok(sessions::Entity::find_by_id(*session_id)
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
        .is_some())
}
