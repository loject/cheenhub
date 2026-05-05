//! Simple in-memory authentication storage.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{RefreshSession, UserAccount};
use crate::features::auth::infrastructure::{AuthStore, InsertUserError, UserConflict};

/// In-memory authentication storage for local runs and tests.
#[derive(Default)]
pub(crate) struct InMemoryAuthStore {
    state: Mutex<InMemoryState>,
}

#[derive(Default)]
struct InMemoryState {
    users: Vec<InMemoryUser>,
    sessions: Vec<InMemorySession>,
    refresh_tokens: Vec<InMemoryRefreshToken>,
}

#[derive(Debug, Clone)]
struct InMemoryUser {
    account: UserAccount,
    email_normalized: String,
}

#[derive(Debug, Clone)]
struct InMemorySession {
    id: Uuid,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct InMemoryRefreshToken {
    id: Uuid,
    session_id: Uuid,
    token_hash: String,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl AuthStore for InMemoryAuthStore {
    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: String,
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
        let state = self.state.lock().map_err(|_| poisoned())?;
        let Some(refresh_token) = state.refresh_tokens.iter().find(|refresh_token| {
            refresh_token.token_hash == token_hash
                && refresh_token.revoked_at.is_none()
                && refresh_token.expires_at > now
        }) else {
            return Ok(None);
        };
        let Some(session) = active_session(&state, &refresh_token.session_id, now) else {
            return Ok(None);
        };
        let Some(user) = state
            .users
            .iter()
            .find(|user| user.account.id == session.user_id)
        else {
            return Ok(None);
        };

        Ok(Some(RefreshSession {
            refresh_token_id: refresh_token.id,
            session_id: session.id,
            user: user.account.clone(),
        }))
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
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(active_session(&state, session_id, now).is_some())
    }
}

fn active_session<'a>(
    state: &'a InMemoryState,
    session_id: &Uuid,
    now: DateTime<Utc>,
) -> Option<&'a InMemorySession> {
    state.sessions.iter().find(|session| {
        session.id == *session_id && session.revoked_at.is_none() && session.expires_at > now
    })
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory auth store lock poisoned")
}
