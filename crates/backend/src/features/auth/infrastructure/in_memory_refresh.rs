//! In-memory refresh token storage helpers.

use std::sync::Mutex;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::RefreshSession;
use crate::features::auth::infrastructure::in_memory::model::{
    InMemoryRefreshToken, InMemorySession, InMemoryState,
};

pub(super) fn create_session(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    refresh_hash: String,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<Uuid> {
    let mut state = state.lock().map_err(|_| poisoned())?;
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

pub(super) fn find_active_refresh(
    state: &Mutex<InMemoryState>,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<RefreshSession>> {
    let state = state.lock().map_err(|_| poisoned())?;
    let Some(refresh_token) = state.refresh_tokens.iter().find(|refresh_token| {
        refresh_token.token_hash == token_hash
            && refresh_token.revoked_at.is_none()
            && refresh_token.expires_at > now
    }) else {
        return Ok(None);
    };
    let Some(session) = state.sessions.iter().find(|session| {
        session.id == refresh_token.session_id
            && session.revoked_at.is_none()
            && session.expires_at > now
    }) else {
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

pub(super) fn session_is_active(
    state: &Mutex<InMemoryState>,
    session_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state.sessions.iter().any(|session| {
        session.id == *session_id && session.revoked_at.is_none() && session.expires_at > now
    }))
}

pub(super) fn rotate_refresh(
    state: &Mutex<InMemoryState>,
    old_refresh_id: &Uuid,
    session_id: &Uuid,
    next_hash: String,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
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

pub(super) fn revoke_refresh_session(
    state: &Mutex<InMemoryState>,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
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

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory auth store lock poisoned")
}
