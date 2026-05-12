//! In-memory refresh token storage helpers.

use std::sync::Mutex;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::RefreshSession;
use crate::features::auth::infrastructure::in_memory::model::InMemoryState;

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

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory auth store lock poisoned")
}
