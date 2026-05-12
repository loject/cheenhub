//! Authentication infrastructure layer.

mod conversions;
mod entities;
mod in_memory;
mod postgres;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState, RefreshSession, UserAccount,
};

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
        password_hash: Option<String>,
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

    /// Inserts a short-lived OAuth state.
    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Consumes an active OAuth state.
    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>>;

    /// Finds a linked OAuth account by provider subject.
    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>>;

    /// Finds a linked OAuth account for a user.
    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>>;

    /// Lists linked OAuth accounts for a user.
    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>>;

    /// Inserts a linked OAuth account.
    async fn insert_oauth_account(
        &self,
        user_id: &Uuid,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<OAuthAccount>;

    /// Deletes a linked OAuth account for a user.
    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool>;

    /// Inserts a short-lived OAuth frontend handoff.
    async fn insert_oauth_handoff(
        &self,
        code_hash: String,
        kind: String,
        user_id: Option<Uuid>,
        registration_intent_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Finds an active OAuth frontend handoff.
    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>>;

    /// Marks an OAuth frontend handoff as consumed.
    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Inserts a short-lived OAuth registration intent.
    async fn insert_oauth_registration_intent(
        &self,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<OAuthRegistrationIntent>;

    /// Finds an active OAuth registration intent.
    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>>;

    /// Marks an OAuth registration intent as consumed.
    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;
}
