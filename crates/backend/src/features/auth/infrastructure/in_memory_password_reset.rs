//! In-memory password reset storage helpers.

use std::sync::Mutex;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::PasswordResetToken;
use crate::features::auth::infrastructure::in_memory::model::{
    InMemoryPasswordResetToken, InMemoryState,
};

pub(super) fn revoke_user_sessions(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let session_ids: Vec<Uuid> = state
        .sessions
        .iter_mut()
        .filter(|session| session.user_id == *user_id && session.revoked_at.is_none())
        .map(|session| {
            session.revoked_at = Some(now);
            session.id
        })
        .collect();

    for refresh_token in &mut state.refresh_tokens {
        if session_ids.contains(&refresh_token.session_id) && refresh_token.revoked_at.is_none() {
            refresh_token.revoked_at = Some(now);
        }
    }

    Ok(())
}

pub(super) fn insert_password_reset_token(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    token_hash: String,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    for token in &mut state.password_reset_tokens {
        if token.user_id == *user_id && token.consumed_at.is_none() {
            token.consumed_at = Some(now);
        }
    }
    state
        .password_reset_tokens
        .push(InMemoryPasswordResetToken {
            id: Uuid::new_v4(),
            user_id: *user_id,
            token_hash,
            expires_at,
            consumed_at: None,
        });

    Ok(())
}

pub(super) fn find_active_password_reset_token(
    state: &Mutex<InMemoryState>,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<PasswordResetToken>> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state
        .password_reset_tokens
        .iter()
        .find(|token| {
            token.token_hash == token_hash && token.consumed_at.is_none() && token.expires_at > now
        })
        .map(|token| PasswordResetToken {
            id: token.id,
            user_id: token.user_id,
        }))
}

pub(super) fn complete_password_reset(
    state: &Mutex<InMemoryState>,
    token_hash: &str,
    password_hash: String,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<PasswordResetToken>> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let Some(token_index) = state.password_reset_tokens.iter().position(|token| {
        token.token_hash == token_hash && token.consumed_at.is_none() && token.expires_at > now
    }) else {
        return Ok(None);
    };
    let token = PasswordResetToken {
        id: state.password_reset_tokens[token_index].id,
        user_id: state.password_reset_tokens[token_index].user_id,
    };
    let Some(user) = state
        .users
        .iter_mut()
        .find(|user| user.account.id == token.user_id)
    else {
        return Err(anyhow!("password reset user is missing"));
    };
    user.account.password_hash = Some(password_hash);
    let session_ids = state
        .sessions
        .iter_mut()
        .filter(|session| session.user_id == token.user_id && session.revoked_at.is_none())
        .map(|session| {
            session.revoked_at = Some(now);
            session.id
        })
        .collect::<Vec<_>>();
    for refresh_token in &mut state.refresh_tokens {
        if session_ids.contains(&refresh_token.session_id) && refresh_token.revoked_at.is_none() {
            refresh_token.revoked_at = Some(now);
        }
    }
    for stored_token in &mut state.password_reset_tokens {
        if stored_token.user_id == token.user_id && stored_token.consumed_at.is_none() {
            stored_token.consumed_at = Some(now);
        }
    }

    Ok(Some(token))
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory auth store lock poisoned")
}
