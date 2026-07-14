use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

pub(super) mod model;

use super::in_memory_refresh as refresh;
use crate::features::auth::domain::*;
use crate::features::auth::infrastructure::*;
use model::*;

#[derive(Default)]
pub(crate) struct InMemoryAuthStore {
    state: Mutex<InMemoryState>,
}

#[async_trait]
impl AuthStore for InMemoryAuthStore {
    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<UserAccount, InsertUserError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| InsertUserError::Storage(poisoned()))?;
        if state
            .users
            .iter()
            .any(|user| user.account.nickname == nickname)
        {
            return Err(InsertUserError::Conflict(UserConflict::Nickname));
        }
        if state
            .users
            .iter()
            .any(|user| user.email_normalized == email_normalized)
        {
            return Err(InsertUserError::Conflict(UserConflict::Email));
        }

        let account = UserAccount {
            id: Uuid::new_v4(),
            nickname,
            email,
            password_hash,
            avatar_image_id: None,
            registered_at: now,
            nickname_updated_at: now,
        };
        state.users.push(InMemoryUser {
            account: account.clone(),
            email_normalized,
        });

        Ok(account)
    }

    async fn find_user_by_email(
        &self,
        email_normalized: &str,
    ) -> anyhow::Result<Option<UserAccount>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .users
            .iter()
            .find(|user| user.email_normalized == email_normalized)
            .map(|user| user.account.clone()))
    }

    async fn find_user_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<UserAccount>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .users
            .iter()
            .find(|user| user.account.id == *user_id)
            .map(|user| user.account.clone()))
    }

    async fn search_users_by_nickname(
        &self,
        query: &str,
        limit: u64,
    ) -> anyhow::Result<Vec<UserAccount>> {
        let needle = query.to_lowercase();
        let limit = usize::try_from(limit).unwrap_or(20);
        let state = self.state.lock().map_err(|_| poisoned())?;
        let mut users = state
            .users
            .iter()
            .filter(|user| user.account.nickname.to_lowercase().contains(&needle))
            .map(|user| user.account.clone())
            .collect::<Vec<_>>();
        users.sort_by(|left, right| left.nickname.cmp(&right.nickname));
        users.truncate(limit);
        Ok(users)
    }

    async fn update_user_nickname(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        nickname: String,
        now: DateTime<Utc>,
        cooldown: Duration,
    ) -> Result<Option<UserAccount>, UpdateUserNicknameError> {
        super::in_memory_profile::update_user_nickname(
            &self.state,
            user_id,
            session_id,
            nickname,
            now,
            cooldown,
        )
    }

    async fn update_user_avatar_image_id(
        &self,
        user_id: &Uuid,
        image_id: Uuid,
        _now: DateTime<Utc>,
    ) -> anyhow::Result<Option<UserAccount>> {
        super::in_memory_profile::update_user_avatar_image_id(&self.state, user_id, image_id)
    }

    async fn avatar_image_ids_by_user_ids(
        &self,
        user_ids: &[Uuid],
    ) -> anyhow::Result<HashMap<Uuid, Uuid>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .users
            .iter()
            .filter(|user| user_ids.contains(&user.account.id))
            .filter_map(|user| {
                user.account
                    .avatar_image_id
                    .map(|image_id| (user.account.id, image_id))
            })
            .collect())
    }

    async fn update_user_password_hash(
        &self,
        user_id: &Uuid,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::in_memory_password_reset::update_user_password_hash(
            &self.state,
            user_id,
            password_hash,
            now,
        )
    }

    async fn change_user_password(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::in_memory_profile::change_user_password(
            &self.state,
            user_id,
            session_id,
            password_hash,
            now,
        )
    }

    async fn create_session(
        &self,
        user_id: &Uuid,
        refresh_hash: String,
        user_agent: Option<&str>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        refresh::create_session(
            &self.state,
            user_id,
            refresh_hash,
            user_agent,
            now,
            expires_at,
        )
    }

    async fn find_active_refresh(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<RefreshSession>> {
        refresh::find_active_refresh(&self.state, token_hash, now)
    }

    async fn rotate_refresh(
        &self,
        old_refresh_id: &Uuid,
        session_id: &Uuid,
        next_hash: String,
        user_agent: Option<&str>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        refresh::rotate_refresh(
            &self.state,
            old_refresh_id,
            session_id,
            next_hash,
            user_agent,
            now,
            expires_at,
        )
    }

    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        refresh::revoke_refresh_session(&self.state, token_hash, now)
    }

    async fn revoke_session_on_refresh_reuse(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        refresh::revoke_session_on_refresh_reuse(&self.state, token_hash, now)
    }

    async fn session_is_active(
        &self,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        refresh::session_is_active(&self.state, session_id, now)
    }

    async fn list_active_sessions(
        &self,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<UserSession>> {
        refresh::list_active_sessions(&self.state, user_id, now)
    }

    async fn record_session_user_agent(
        &self,
        session_id: &Uuid,
        user_agent: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        refresh::record_session_user_agent(&mut state, *session_id, user_agent, now);
        Ok(())
    }

    async fn revoke_user_session(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        refresh::revoke_user_session(&self.state, user_id, session_id, now)
    }

    async fn revoke_user_sessions(&self, user_id: &Uuid, now: DateTime<Utc>) -> anyhow::Result<()> {
        super::in_memory_password_reset::revoke_user_sessions(&self.state, user_id, now)
    }

    async fn insert_password_reset_token(
        &self,
        user_id: &Uuid,
        token_hash: String,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::in_memory_password_reset::insert_password_reset_token(
            &self.state,
            user_id,
            token_hash,
            expires_at,
        )
    }

    async fn consume_password_reset_token(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<PasswordResetToken>> {
        super::in_memory_password_reset::consume_password_reset_token(&self.state, token_hash, now)
    }

    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::in_memory_oauth::insert_oauth_state(
            &self.state,
            state_hash,
            nonce,
            flow_kind,
            user_id,
            expires_at,
        )
    }

    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>> {
        super::in_memory_oauth::consume_oauth_state(&self.state, state_hash, now)
    }

    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::in_memory_oauth::find_oauth_account_by_subject(
            &self.state,
            provider,
            provider_subject,
        )
    }

    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::in_memory_oauth::find_oauth_account_for_user(&self.state, provider, user_id)
    }

    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>> {
        super::in_memory_oauth::list_oauth_accounts(&self.state, user_id)
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
        super::in_memory_oauth::insert_oauth_account(
            &self.state,
            user_id,
            provider,
            provider_subject,
            email,
            display_name,
            now,
        )
    }

    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool> {
        super::in_memory_oauth::delete_oauth_account(&self.state, provider, user_id)
    }

    async fn insert_oauth_handoff(
        &self,
        code_hash: String,
        kind: String,
        user_id: Option<Uuid>,
        registration_intent_id: Option<Uuid>,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::in_memory_oauth::insert_oauth_handoff(
            &self.state,
            code_hash,
            kind,
            user_id,
            registration_intent_id,
            expires_at,
        )
    }

    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>> {
        super::in_memory_oauth::find_active_oauth_handoff(&self.state, code_hash, now)
    }

    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::in_memory_oauth::consume_oauth_handoff(&self.state, handoff_id, now)
    }

    async fn insert_oauth_registration_intent(
        &self,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<OAuthRegistrationIntent> {
        super::in_memory_oauth::insert_oauth_registration_intent(
            &self.state,
            provider,
            provider_subject,
            email,
            display_name,
            expires_at,
        )
    }

    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
        super::in_memory_oauth::find_active_oauth_registration_intent(&self.state, intent_id, now)
    }

    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::in_memory_oauth::consume_oauth_registration_intent(&self.state, intent_id, now)
    }
}

pub(super) fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory auth store lock poisoned")
}
