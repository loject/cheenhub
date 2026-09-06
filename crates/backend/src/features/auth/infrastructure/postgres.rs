//! Postgres-backed authentication storage.

use super::postgres_store::PostgresAuthStore;
use super::{postgres_desktop_oauth as desktop, postgres_user as user};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::features::auth::domain::*;
use crate::features::auth::infrastructure::{AuthStore, InsertUserError, UpdateUserNicknameError};

#[async_trait]
impl AuthStore for PostgresAuthStore {
    async fn insert_desktop_oauth_attempt(
        &self,
        attempt: DesktopOAuthAttempt,
    ) -> anyhow::Result<()> {
        desktop::insert(&self.database, attempt).await
    }

    async fn desktop_oauth_attempt_by_state_hash(
        &self,
        state_hash: &str,
    ) -> anyhow::Result<Option<Uuid>> {
        desktop::find_by_state(&self.database, state_hash).await
    }

    async fn desktop_oauth_status(
        &self,
        id: &Uuid,
        secret_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<DesktopOAuthStatus>> {
        desktop::status(&self.database, id, secret_hash, now).await
    }

    async fn desktop_oauth_attempt_is_pending(
        &self,
        id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        desktop::is_pending(&self.database, id, now).await
    }

    async fn finish_desktop_oauth_attempt(
        &self,
        id: &Uuid,
        kind: String,
        user_id: Option<Uuid>,
        identity: DesktopOAuthIdentity,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        desktop::finish(&self.database, id, kind, user_id, identity, now).await
    }

    async fn desktop_oauth_identity_for_handoff(
        &self,
        handoff_id: &Uuid,
    ) -> anyhow::Result<Option<DesktopOAuthIdentity>> {
        desktop::identity_for_handoff(&self.database, handoff_id).await
    }

    async fn fail_desktop_oauth_attempt(
        &self,
        id: &Uuid,
        message: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        desktop::fail(&self.database, id, message, now).await
    }

    async fn cancel_desktop_oauth_attempt(
        &self,
        id: &Uuid,
        secret_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        desktop::cancel(&self.database, id, secret_hash, now).await
    }

    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: Option<String>,
        legal_acceptance: RegistrationLegalAcceptance,
        now: DateTime<Utc>,
    ) -> Result<UserAccount, InsertUserError> {
        user::insert_user(
            &self.database,
            nickname,
            email,
            email_normalized,
            password_hash,
            legal_acceptance,
            now,
        )
        .await
    }

    async fn find_user_by_email(
        &self,
        email_normalized: &str,
    ) -> anyhow::Result<Option<UserAccount>> {
        user::find_user_by_email(&self.database, email_normalized).await
    }

    async fn find_user_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<UserAccount>> {
        user::find_user_by_id(&self.database, user_id).await
    }

    async fn search_users_by_nickname(
        &self,
        query: &str,
        limit: u64,
    ) -> anyhow::Result<Vec<UserAccount>> {
        user::search_users_by_nickname(&self.database, query, limit).await
    }

    async fn update_user_nickname(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        nickname: String,
        now: DateTime<Utc>,
        cooldown: Duration,
    ) -> Result<Option<UserAccount>, UpdateUserNicknameError> {
        super::postgres_profile::update_user_nickname(
            &self.database,
            user_id,
            session_id,
            nickname,
            now,
            cooldown,
        )
        .await
    }

    async fn update_user_avatar_image_id(
        &self,
        user_id: &Uuid,
        image_id: Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<UserAccount>> {
        user::update_user_avatar_image_id(&self.database, user_id, image_id, now).await
    }

    async fn avatar_image_ids_by_user_ids(
        &self,
        user_ids: &[Uuid],
    ) -> anyhow::Result<HashMap<Uuid, Uuid>> {
        user::avatar_image_ids_by_user_ids(&self.database, user_ids).await
    }

    async fn change_user_password(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::postgres_profile::change_user_password(
            &self.database,
            user_id,
            session_id,
            password_hash,
            now,
        )
        .await
    }

    async fn create_session(
        &self,
        user_id: &Uuid,
        refresh_hash: String,
        user_agent: Option<&str>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        super::postgres_refresh::create_session(
            &self.database,
            user_id,
            refresh_hash,
            user_agent,
            now,
            expires_at,
        )
        .await
    }

    async fn find_active_refresh(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<RefreshSession>> {
        super::postgres_refresh::find_active_refresh(&self.database, token_hash, now).await
    }

    async fn rotate_refresh(
        &self,
        old_refresh_id: &Uuid,
        session_id: &Uuid,
        next_hash: String,
        user_agent: Option<&str>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<super::RotateRefreshOutcome> {
        super::postgres_refresh::rotate_refresh(
            &self.database,
            old_refresh_id,
            session_id,
            next_hash,
            user_agent,
            now,
            expires_at,
        )
        .await
    }

    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>> {
        super::postgres_refresh::revoke_refresh_session(&self.database, token_hash, now).await
    }

    async fn revoke_session_on_refresh_reuse(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
        concurrent_rotation_after: DateTime<Utc>,
    ) -> anyhow::Result<super::RefreshReuseOutcome> {
        super::postgres_refresh::revoke_session_on_refresh_reuse(
            &self.database,
            token_hash,
            now,
            concurrent_rotation_after,
        )
        .await
    }

    async fn session_is_active(
        &self,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        super::postgres_refresh::session_is_active(&self.database, session_id, now).await
    }

    async fn list_active_sessions(
        &self,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<UserSession>> {
        super::postgres_refresh::list_active_sessions(&self.database, user_id, now).await
    }

    async fn record_session_user_agent(
        &self,
        session_id: &Uuid,
        user_agent: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::postgres_refresh::record_session_user_agent(
            &self.database,
            session_id,
            user_agent,
            now,
        )
        .await
    }

    async fn revoke_user_session(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        super::postgres_refresh::revoke_user_session(&self.database, user_id, session_id, now).await
    }

    async fn revoke_user_sessions(&self, user_id: &Uuid, now: DateTime<Utc>) -> anyhow::Result<()> {
        super::postgres_password_reset::revoke_user_sessions(&self.database, user_id, now).await
    }

    async fn insert_password_reset_token(
        &self,
        user_id: &Uuid,
        token_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::postgres_password_reset::insert_password_reset_token(
            &self.database,
            user_id,
            token_hash,
            now,
            expires_at,
        )
        .await
    }

    async fn find_active_password_reset_token(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<crate::features::auth::domain::PasswordResetToken>> {
        super::postgres_password_reset::find_active_password_reset_token(
            &self.database,
            token_hash,
            now,
        )
        .await
    }

    async fn complete_password_reset(
        &self,
        token_hash: &str,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<crate::features::auth::domain::PasswordResetToken>> {
        super::postgres_password_reset::complete_password_reset(
            &self.database,
            token_hash,
            password_hash,
            now,
        )
        .await
    }

    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        super::postgres_oauth::insert_oauth_state(
            &self.database,
            state_hash,
            nonce,
            flow_kind,
            user_id,
            now,
            expires_at,
        )
        .await
    }

    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>> {
        super::postgres_oauth::consume_oauth_state(&self.database, state_hash, now).await
    }

    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::postgres_oauth::find_oauth_account_by_subject(
            &self.database,
            provider,
            provider_subject,
        )
        .await
    }

    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::postgres_oauth::find_oauth_account_for_user(&self.database, provider, user_id).await
    }

    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>> {
        super::postgres_oauth::list_oauth_accounts(&self.database, user_id).await
    }

    async fn insert_oauth_account(
        &self,
        user_id: &Uuid,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<OAuthAccount> {
        super::postgres_oauth::insert_oauth_account(
            &self.database,
            user_id,
            provider,
            provider_subject,
            email,
            display_name,
            now,
        )
        .await
    }

    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool> {
        super::postgres_oauth::delete_oauth_account(&self.database, provider, user_id).await
    }

    async fn insert_oauth_handoff(
        &self,
        code_hash: String,
        kind: String,
        user_id: Option<Uuid>,
        registration_intent_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::postgres_oauth::insert_oauth_handoff(
            &self.database,
            code_hash,
            kind,
            user_id,
            registration_intent_id,
            now,
            expires_at,
        )
        .await
    }

    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>> {
        super::postgres_oauth::find_active_oauth_handoff(&self.database, code_hash, now).await
    }

    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        super::postgres_oauth::consume_oauth_handoff(&self.database, handoff_id, now).await
    }

    async fn insert_oauth_registration_intent(
        &self,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<OAuthRegistrationIntent> {
        super::postgres_oauth::insert_oauth_registration_intent(
            &self.database,
            provider,
            provider_subject,
            email,
            display_name,
            now,
            expires_at,
        )
        .await
    }

    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
        super::postgres_oauth::find_active_oauth_registration_intent(&self.database, intent_id, now)
            .await
    }

    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::postgres_oauth::consume_oauth_registration_intent(&self.database, intent_id, now)
            .await
    }
}
