//! Простое in-memory хранение tombstone для разработки и тестов.
use super::in_memory::{model::InMemoryState, poisoned};
use crate::features::auth::domain::AccountDeletion;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use uuid::Uuid;

pub(super) fn account_deletion(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
) -> anyhow::Result<Option<AccountDeletion>> {
    Ok(state
        .lock()
        .map_err(|_| poisoned())?
        .users
        .iter()
        .find(|user| user.account.id == *user_id)
        .and_then(|user| user.deletion.clone()))
}
pub(super) fn begin_account_deletion(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    token_hash: String,
    now: DateTime<Utc>,
    restore_until: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let Some(user) = state
        .users
        .iter_mut()
        .find(|user| user.account.id == *user_id && user.deletion.is_none())
    else {
        return Ok(false);
    };
    user.deletion = Some(AccountDeletion {
        requested_at: now,
        restore_until,
    });
    user.deletion_token_hash = Some(token_hash);
    user.deletion_finalized_at = None;
    for session in state
        .sessions
        .iter_mut()
        .filter(|row| row.user_id == *user_id)
    {
        session.revoked_at = Some(now);
    }
    for token in state
        .password_reset_tokens
        .iter_mut()
        .filter(|row| row.user_id == *user_id)
    {
        token.consumed_at = Some(now);
    }
    tracing::info!(%user_id, %restore_until, "in-memory account tombstone created");
    Ok(true)
}
pub(super) fn restore_account(
    state: &Mutex<InMemoryState>,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    // Учитываем время, прошедшее в ожидании блокировки.
    let now = now.max(Utc::now());
    let Some(user) = state.users.iter_mut().find(|user| {
        user.deletion_token_hash.as_deref() == Some(token_hash)
            && user
                .deletion
                .as_ref()
                .is_some_and(|deletion| deletion.restore_until > now)
            && user.deletion_finalized_at.is_none()
    }) else {
        return Ok(false);
    };
    user.deletion = None;
    user.deletion_token_hash = None;
    user.deletion_finalized_at = None;
    tracing::info!(user_id = %user.account.id, "in-memory account restored");
    Ok(true)
}
pub(super) fn finalize_expired_account_deletions(
    state: &Mutex<InMemoryState>,
    now: DateTime<Utc>,
) -> anyhow::Result<u64> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let ids: Vec<_> = state
        .users
        .iter()
        .filter(|user| {
            user.deletion
                .as_ref()
                .is_some_and(|deletion| deletion.restore_until <= now)
                && user.deletion_finalized_at.is_none()
        })
        .map(|user| user.account.id)
        .collect();
    for id in &ids {
        if let Some(user) = state.users.iter_mut().find(|user| user.account.id == *id) {
            user.account.nickname = format!("deleted:{}", URL_SAFE_NO_PAD.encode(id.as_bytes()));
            user.account.email = format!("deleted:{id}");
            user.email_normalized = user.account.email.clone();
            user.account.password_hash = None;
            user.account.avatar_image_id = None;
            user.deletion_token_hash = None;
            user.deletion_finalized_at = Some(now);
        }
        state.oauth_accounts.retain(|row| row.user_id != *id);
        state.oauth_states.retain(|row| row.user_id != Some(*id));
        state.oauth_handoffs.retain(|row| row.user_id != Some(*id));
        state.password_reset_tokens.retain(|row| row.user_id != *id);
        state.user_password_change_trace.retain(|row| row.1 != *id);
        state.user_nickname_history.retain(|row| row.1 != *id);
        let session_ids: Vec<_> = state
            .sessions
            .iter()
            .filter(|row| row.user_id == *id)
            .map(|row| row.id)
            .collect();
        state.sessions.retain(|row| row.user_id != *id);
        state
            .refresh_tokens
            .retain(|row| !session_ids.contains(&row.session_id));
        state
            .session_user_agents
            .retain(|row| !session_ids.contains(&row.session_id));
        tracing::info!(user_id = %id, "in-memory expired account anonymized");
    }
    Ok(ids.len() as u64)
}
