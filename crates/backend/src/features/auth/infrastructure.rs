//! Authentication infrastructure layer.

mod entities;
mod in_memory;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use uuid::Uuid;

use crate::features::auth::domain::{RefreshSession, UserAccount};
use crate::features::auth::infrastructure::entities::{refresh_tokens, sessions, users};

pub(crate) use in_memory::InMemoryAuthStore;

/// Unique user field conflict.
#[derive(Debug)]
pub(crate) enum UserConflict {
    /// Nickname is already used.
    Nickname,
    /// Email is already used.
    Email,
}

/// Error returned while inserting a user.
#[derive(Debug)]
pub(crate) enum InsertUserError {
    /// Unique field conflict.
    Conflict(UserConflict),
    /// Unexpected database error.
    Database(sea_orm::DbErr),
    /// Unexpected storage error.
    Storage(anyhow::Error),
}

/// Authentication storage boundary.
#[async_trait]
pub(crate) trait AuthStore: Send + Sync {
    /// Inserts a new user account.
    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> Result<UserAccount, InsertUserError>;

    /// Finds a user by normalized email.
    async fn find_user_by_email(
        &self,
        email_normalized: &str,
    ) -> anyhow::Result<Option<UserAccount>>;

    /// Finds a user by id.
    async fn find_user_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<UserAccount>>;

    /// Creates a session and its initial refresh token row.
    async fn create_session(
        &self,
        user_id: &Uuid,
        refresh_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid>;

    /// Finds an active refresh session by token hash.
    async fn find_active_refresh(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<RefreshSession>>;

    /// Rotates a refresh token for an existing session.
    async fn rotate_refresh(
        &self,
        old_refresh_id: &Uuid,
        session_id: &Uuid,
        next_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Revokes a refresh token and the session that owns it.
    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Returns whether a session is currently active.
    async fn session_is_active(
        &self,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;
}

/// Postgres-backed authentication storage.
pub(crate) struct PostgresAuthStore {
    database: DatabaseConnection,
}

impl PostgresAuthStore {
    /// Builds a Postgres-backed authentication storage.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl AuthStore for PostgresAuthStore {
    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> Result<UserAccount, InsertUserError> {
        insert_user(
            &self.database,
            nickname,
            email,
            email_normalized,
            password_hash,
            now,
        )
        .await
    }

    async fn find_user_by_email(
        &self,
        email_normalized: &str,
    ) -> anyhow::Result<Option<UserAccount>> {
        find_user_by_email(&self.database, email_normalized).await
    }

    async fn find_user_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<UserAccount>> {
        find_user_by_id(&self.database, user_id).await
    }

    async fn create_session(
        &self,
        user_id: &Uuid,
        refresh_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        create_session(&self.database, user_id, refresh_hash, now, expires_at).await
    }

    async fn find_active_refresh(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<RefreshSession>> {
        find_active_refresh(&self.database, token_hash, now).await
    }

    async fn rotate_refresh(
        &self,
        old_refresh_id: &Uuid,
        session_id: &Uuid,
        next_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        rotate_refresh(
            &self.database,
            old_refresh_id,
            session_id,
            next_hash,
            now,
            expires_at,
        )
        .await
    }

    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        revoke_refresh_session(&self.database, token_hash, now).await
    }

    async fn session_is_active(
        &self,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        session_is_active(&self.database, session_id, now).await
    }
}

/// Inserts a new user account.
async fn insert_user(
    database: &impl ConnectionTrait,
    nickname: String,
    email: String,
    email_normalized: String,
    password_hash: String,
    now: DateTime<Utc>,
) -> Result<UserAccount, InsertUserError> {
    let user_id = Uuid::new_v4();
    let model = users::ActiveModel {
        id: Set(user_id),
        nickname: Set(nickname),
        email: Set(email),
        email_normalized: Set(email_normalized),
        password_hash: Set(password_hash),
        registered_at: Set(now),
        accepted_terms_at: Set(now),
        updated_at: Set(now),
    }
    .insert(database)
    .await
    .map_err(map_insert_user_error)?;

    Ok(model.into())
}

/// Finds a user by normalized email.
async fn find_user_by_email(
    database: &impl ConnectionTrait,
    email_normalized: &str,
) -> anyhow::Result<Option<UserAccount>> {
    Ok(users::Entity::find()
        .filter(users::Column::EmailNormalized.eq(email_normalized))
        .one(database)
        .await?
        .map(Into::into))
}

/// Finds a user by id.
async fn find_user_by_id(
    database: &impl ConnectionTrait,
    user_id: &Uuid,
) -> anyhow::Result<Option<UserAccount>> {
    Ok(users::Entity::find_by_id(*user_id)
        .one(database)
        .await?
        .map(Into::into))
}

/// Creates a session and its initial refresh token row.
async fn create_session(
    database: &impl ConnectionTrait,
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

/// Finds an active refresh session by token hash.
async fn find_active_refresh(
    database: &impl ConnectionTrait,
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

/// Rotates a refresh token for an existing session.
async fn rotate_refresh(
    database: &impl ConnectionTrait,
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

/// Revokes a refresh token and the session that owns it.
async fn revoke_refresh_session(
    database: &impl ConnectionTrait,
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

/// Returns whether a session is currently active.
async fn session_is_active(
    database: &impl ConnectionTrait,
    session_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    Ok(sessions::Entity::find_by_id(session_id.to_owned())
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
        .is_some())
}

impl From<users::Model> for UserAccount {
    fn from(row: users::Model) -> Self {
        Self {
            id: row.id,
            nickname: row.nickname,
            email: row.email,
            password_hash: row.password_hash,
            registered_at: row.registered_at,
        }
    }
}

fn map_insert_user_error(error: sea_orm::DbErr) -> InsertUserError {
    let message = error.to_string();
    if message.contains("users_nickname_key") {
        return InsertUserError::Conflict(UserConflict::Nickname);
    }
    if message.contains("users_email_normalized_key") {
        return InsertUserError::Conflict(UserConflict::Email);
    }

    InsertUserError::Database(error)
}
