//! Postgres refresh token storage helpers.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::features::auth::domain::{RefreshSession, UserSession};
use crate::features::auth::infrastructure::entities::{
    refresh_tokens, session_user_agents, sessions, users,
};
use crate::features::auth::infrastructure::{RefreshReuseOutcome, RotateRefreshOutcome};
use crate::features::auth::security::user_agent;

pub(super) async fn create_session(
    database: &DatabaseConnection,
    user_id: &Uuid,
    refresh_hash: String,
    user_agent: Option<&str>,
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

    if let Some(user_agent) = user_agent {
        record_session_user_agent(database, &session_id, user_agent, now).await?;
    }

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

pub(super) async fn list_active_sessions(
    database: &DatabaseConnection,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<Vec<UserSession>> {
    let sessions = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(*user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .order_by_desc(sessions::Column::LastSeenAt)
        .all(database)
        .await?;

    if sessions.is_empty() {
        return Ok(Vec::new());
    }

    let session_ids = sessions
        .iter()
        .map(|session| session.id)
        .collect::<Vec<_>>();
    let mut latest_user_agents = HashMap::<Uuid, String>::new();
    for observed in session_user_agents::Entity::find()
        .filter(session_user_agents::Column::SessionId.is_in(session_ids))
        .order_by_desc(session_user_agents::Column::LastSeenAt)
        .all(database)
        .await?
    {
        latest_user_agents
            .entry(observed.session_id)
            .or_insert(observed.user_agent);
    }

    Ok(sessions
        .into_iter()
        .map(|session| UserSession {
            id: session.id,
            created_at: session.created_at,
            last_seen_at: session.last_seen_at,
            expires_at: session.expires_at,
            user_agent: latest_user_agents.remove(&session.id),
        })
        .collect())
}

pub(super) async fn rotate_refresh(
    database: &DatabaseConnection,
    old_refresh_id: &Uuid,
    session_id: &Uuid,
    next_hash: String,
    user_agent: Option<&str>,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<RotateRefreshOutcome> {
    let transaction = database.begin().await?;
    let consumed = refresh_tokens::Entity::update_many()
        .col_expr(
            refresh_tokens::Column::RotatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .col_expr(
            refresh_tokens::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(refresh_tokens::Column::Id.eq(*old_refresh_id))
        .filter(refresh_tokens::Column::SessionId.eq(*session_id))
        .filter(refresh_tokens::Column::RevokedAt.is_null())
        .filter(refresh_tokens::Column::ExpiresAt.gt(now))
        .exec(&transaction)
        .await?;
    if consumed.rows_affected != 1 {
        transaction.rollback().await?;
        return Ok(RotateRefreshOutcome::AlreadyConsumed);
    }

    let updated_session = sessions::Entity::update_many()
        .col_expr(
            sessions::Column::LastSeenAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .col_expr(
            sessions::Column::ExpiresAt,
            sea_orm::sea_query::Expr::value(expires_at),
        )
        .filter(sessions::Column::Id.eq(*session_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .exec(&transaction)
        .await?;
    if updated_session.rows_affected != 1 {
        transaction.rollback().await?;
        return Ok(RotateRefreshOutcome::AlreadyConsumed);
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
    .insert(&transaction)
    .await?;

    if let Some(user_agent) = user_agent {
        record_session_user_agent(&transaction, session_id, user_agent, now).await?;
    }

    transaction.commit().await?;
    Ok(RotateRefreshOutcome::Rotated)
}

pub(super) async fn revoke_user_session(
    database: &DatabaseConnection,
    user_id: &Uuid,
    session_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let result = sessions::Entity::update_many()
        .col_expr(
            sessions::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(sessions::Column::Id.eq(*session_id))
        .filter(sessions::Column::UserId.eq(*user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .exec(database)
        .await?;

    if result.rows_affected == 0 {
        return Ok(false);
    }

    refresh_tokens::Entity::update_many()
        .col_expr(
            refresh_tokens::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(refresh_tokens::Column::SessionId.eq(*session_id))
        .filter(refresh_tokens::Column::RevokedAt.is_null())
        .exec(database)
        .await?;

    Ok(true)
}

pub(super) async fn revoke_session_on_refresh_reuse(
    database: &DatabaseConnection,
    token_hash: &str,
    now: DateTime<Utc>,
    concurrent_rotation_after: DateTime<Utc>,
) -> anyhow::Result<RefreshReuseOutcome> {
    let Some(refresh_token) = refresh_tokens::Entity::find()
        .filter(refresh_tokens::Column::TokenHash.eq(token_hash))
        .one(database)
        .await?
    else {
        return Ok(RefreshReuseOutcome::NotDetected);
    };

    // Активный (не отозванный) токен — это просто просрочка или опечатка, не кража.
    if refresh_token.revoked_at.is_none() {
        return Ok(RefreshReuseOutcome::NotDetected);
    }
    if refresh_token.rotated_at.is_none() {
        return Ok(RefreshReuseOutcome::SessionRevoked);
    }
    if refresh_token
        .rotated_at
        .is_some_and(|rotated_at| rotated_at >= concurrent_rotation_after)
    {
        return Ok(RefreshReuseOutcome::ConcurrentRotation);
    }

    // Токен уже был ротирован/отозван, но предъявлен снова — вероятная компрометация.
    // Завершаем всю сессию и все ее refresh-токены.
    let session_id = refresh_token.session_id;
    sessions::Entity::update_many()
        .col_expr(
            sessions::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(sessions::Column::Id.eq(session_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .exec(database)
        .await?;
    refresh_tokens::Entity::update_many()
        .col_expr(
            refresh_tokens::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(refresh_tokens::Column::SessionId.eq(session_id))
        .filter(refresh_tokens::Column::RevokedAt.is_null())
        .exec(database)
        .await?;

    Ok(RefreshReuseOutcome::ReusedAndRevoked)
}

pub(super) async fn record_session_user_agent<C: ConnectionTrait>(
    database: &C,
    session_id: &Uuid,
    user_agent: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let Some(user_agent) = user_agent::normalize(user_agent) else {
        return Ok(());
    };

    if let Some(existing) = session_user_agents::Entity::find()
        .filter(session_user_agents::Column::SessionId.eq(*session_id))
        .filter(session_user_agents::Column::UserAgent.eq(&user_agent))
        .one(database)
        .await?
    {
        let mut existing = existing.into_active_model();
        existing.last_seen_at = Set(now);
        existing.update(database).await?;
        tracing::debug!(%session_id, "updated auth session user-agent observation");
        return Ok(());
    }

    session_user_agents::ActiveModel {
        id: Set(Uuid::new_v4()),
        session_id: Set(*session_id),
        user_agent: Set(user_agent),
        first_seen_at: Set(now),
        last_seen_at: Set(now),
    }
    .insert(database)
    .await?;
    tracing::info!(%session_id, "recorded new auth session user-agent");

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
