use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

pub(super) mod model;

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

    #[rustfmt::skip]
    async fn update_user_nickname(
        &self, user_id: &Uuid, session_id: &Uuid, nickname: String, now: DateTime<Utc>, cooldown: Duration,
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

    async fn create_session(
        &self,
        user_id: &Uuid,
        refresh_hash: String,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let session_id = Uuid::new_v4();

        state.sessions.push(InMemorySession {
            id: session_id,
            user_id: *user_id,
            expires_at,
            revoked_at: None,
        });
        state.refresh_tokens.push(InMemoryRefreshToken {
            id: Uuid::new_v4(),
            session_id,
            token_hash: refresh_hash,
            expires_at,
            revoked_at: None,
        });

        Ok(session_id)
    }

    async fn find_active_refresh(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<RefreshSession>> {
        super::in_memory_refresh::find_active_refresh(&self.state, token_hash, now)
    }

    async fn rotate_refresh(
        &self,
        old_refresh_id: &Uuid,
        session_id: &Uuid,
        next_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        if let Some(refresh_token) = state
            .refresh_tokens
            .iter_mut()
            .find(|refresh_token| refresh_token.id == *old_refresh_id)
        {
            refresh_token.revoked_at = Some(now);
        }
        if let Some(session) = state
            .sessions
            .iter_mut()
            .find(|session| session.id == *session_id && session.revoked_at.is_none())
        {
            session.expires_at = expires_at;
        }
        state.refresh_tokens.push(InMemoryRefreshToken {
            id: Uuid::new_v4(),
            session_id: *session_id,
            token_hash: next_hash,
            expires_at,
            revoked_at: None,
        });

        Ok(())
    }

    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let Some(refresh_token) = state
            .refresh_tokens
            .iter_mut()
            .find(|refresh_token| refresh_token.token_hash == token_hash)
        else {
            return Ok(());
        };
        refresh_token.revoked_at = Some(now);
        let session_id = refresh_token.session_id;

        if let Some(session) = state
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        {
            session.revoked_at = Some(now);
        }

        Ok(())
    }

    async fn session_is_active(
        &self,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        super::in_memory_refresh::session_is_active(&self.state, session_id, now)
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
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        state.oauth_states.push(InMemoryOAuthState {
            state_hash,
            nonce,
            flow_kind,
            user_id,
            expires_at,
            consumed_at: None,
        });

        Ok(())
    }

    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let Some(oauth_state) = state.oauth_states.iter_mut().find(|oauth_state| {
            oauth_state.state_hash == state_hash
                && oauth_state.consumed_at.is_none()
                && oauth_state.expires_at > now
        }) else {
            return Ok(None);
        };
        oauth_state.consumed_at = Some(now);

        Ok(Some(OAuthState {
            nonce: oauth_state.nonce.clone(),
            flow_kind: oauth_state.flow_kind.clone(),
            user_id: oauth_state.user_id,
        }))
    }

    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .oauth_accounts
            .iter()
            .find(|account| {
                account.provider == provider && account.provider_subject == provider_subject
            })
            .cloned())
    }

    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .oauth_accounts
            .iter()
            .find(|account| account.provider == provider && account.user_id == *user_id)
            .cloned())
    }

    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .oauth_accounts
            .iter()
            .filter(|account| account.user_id == *user_id)
            .cloned()
            .collect())
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
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        if state.oauth_accounts.iter().any(|account| {
            account.provider == provider && account.provider_subject == provider_subject
        }) {
            return Err(anyhow::anyhow!("oauth provider subject is already linked"));
        }
        if state
            .oauth_accounts
            .iter()
            .any(|account| account.provider == provider && account.user_id == *user_id)
        {
            return Err(anyhow::anyhow!("oauth provider is already linked for user"));
        }
        let account = OAuthAccount {
            user_id: *user_id,
            provider,
            provider_subject,
            email,
            display_name,
            linked_at: now,
        };
        state.oauth_accounts.push(account.clone());

        Ok(account)
    }

    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let previous_len = state.oauth_accounts.len();
        state
            .oauth_accounts
            .retain(|account| account.provider != provider || account.user_id != *user_id);

        Ok(state.oauth_accounts.len() != previous_len)
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
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        state.oauth_handoffs.push(InMemoryOAuthHandoff {
            id: Uuid::new_v4(),
            code_hash,
            kind,
            user_id,
            registration_intent_id,
            expires_at,
            consumed_at: None,
        });

        Ok(())
    }

    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .oauth_handoffs
            .iter()
            .find(|handoff| {
                handoff.code_hash == code_hash
                    && handoff.consumed_at.is_none()
                    && handoff.expires_at > now
            })
            .map(|handoff| OAuthHandoff {
                id: handoff.id,
                kind: handoff.kind.clone(),
                user_id: handoff.user_id,
                registration_intent_id: handoff.registration_intent_id,
            }))
    }

    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        if let Some(handoff) = state
            .oauth_handoffs
            .iter_mut()
            .find(|handoff| handoff.id == *handoff_id)
        {
            handoff.consumed_at = Some(now);
        }

        Ok(())
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
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        let intent = OAuthRegistrationIntent {
            id: Uuid::new_v4(),
            provider_subject,
            email,
            display_name,
        };
        state
            .oauth_registration_intents
            .push(InMemoryOAuthRegistrationIntent {
                intent: intent.clone(),
                provider,
                expires_at,
                consumed_at: None,
            });

        Ok(intent)
    }

    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state
            .oauth_registration_intents
            .iter()
            .find(|intent| {
                intent.intent.id == *intent_id
                    && intent.provider == "google"
                    && intent.consumed_at.is_none()
                    && intent.expires_at > now
            })
            .map(|intent| intent.intent.clone()))
    }

    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        if let Some(intent) = state
            .oauth_registration_intents
            .iter_mut()
            .find(|intent| intent.intent.id == *intent_id)
        {
            intent.consumed_at = Some(now);
        }

        Ok(())
    }
}

pub(super) fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory auth store lock poisoned")
}
