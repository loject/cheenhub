//! Authentication infrastructure layer.

mod entities;
mod in_memory;
mod postgres;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{RefreshSession, UserAccount};

pub(crate) use in_memory::InMemoryAuthStore;
pub(crate) use postgres::PostgresAuthStore;

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
