//! Postgres refresh token storage helpers.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::features::auth::domain::RefreshSession;
use crate::features::auth::infrastructure::entities::{refresh_tokens, sessions, users};

pub(super) async fn create_session(
    database: &DatabaseConnection,
    user_id: &Uuid,
    refresh_hash: String,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<Uuid> {
    let session_id = Uuid::new_v4();
    let refresh_id = Uuid::new_v4();

    sessions::ActiveModel {
        id: Set(session_id),
        user_id: Set(*user_id),
        created_at: Set(now),
        last_seen_at: Set(now),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
    }
    .insert(database)
    .await?;

    refresh_tokens::ActiveModel {
        id: Set(refresh_id),
        session_id: Set(session_id),
        token_hash: Set(refresh_hash),
        created_at: Set(now),
        rotated_at: Set(None),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
    }
    .insert(database)
    .await?;

    Ok(session_id)
}

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

pub(super) async fn rotate_refresh(
    database: &DatabaseConnection,
    old_refresh_id: &Uuid,
    session_id: &Uuid,
    next_hash: String,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    if let Some(old_refresh) = refresh_tokens::Entity::find_by_id(old_refresh_id.to_owned())
        .one(database)
        .await?
    {
        let mut old_refresh = old_refresh.into_active_model();
        old_refresh.rotated_at = Set(Some(now));
        old_refresh.revoked_at = Set(Some(now));
        old_refresh.update(database).await?;
    }

    if let Some(session) = sessions::Entity::find_by_id(session_id.to_owned())
        .filter(sessions::Column::RevokedAt.is_null())
        .one(database)
        .await?
    {
        let mut session = session.into_active_model();
        session.last_seen_at = Set(now);
        session.expires_at = Set(expires_at);
        session.update(database).await?;
    }

    refresh_tokens::ActiveModel {
        id: Set(Uuid::new_v4()),
        session_id: Set(*session_id),
        token_hash: Set(next_hash),
        created_at: Set(now),
        rotated_at: Set(None),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
    }
    .insert(database)
    .await?;

    Ok(())
}

pub(super) async fn revoke_refresh_session(
    database: &DatabaseConnection,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let Some(refresh_token) = refresh_tokens::Entity::find()
        .filter(refresh_tokens::Column::TokenHash.eq(token_hash))
        .one(database)
        .await?
    else {
        return Ok(());
    };
    let session_id = refresh_token.session_id;

    if refresh_token.revoked_at.is_none() {
        let mut refresh_token = refresh_token.into_active_model();
        refresh_token.revoked_at = Set(Some(now));
        refresh_token.update(database).await?;
    }

    if let Some(session) = sessions::Entity::find_by_id(session_id)
        .filter(sessions::Column::RevokedAt.is_null())
        .one(database)
        .await?
    {
        let mut session = session.into_active_model();
        session.revoked_at = Set(Some(now));
        session.update(database).await?;
    }

    Ok(())
}
