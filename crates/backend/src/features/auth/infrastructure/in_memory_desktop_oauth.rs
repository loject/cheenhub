//! Атомарные операции desktop OAuth под общей блокировкой тестового хранилища.

use super::in_memory::{
    model::{InMemoryDesktopOAuthAttempt, InMemoryOAuthHandoff, InMemoryState},
    poisoned,
};
use crate::features::auth::domain::{
    DesktopOAuthAttempt, DesktopOAuthIdentity, DesktopOAuthStatus,
};
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use uuid::Uuid;

/// Сохраняет попытку, связанную с существующим состоянием OAuth.
pub(super) fn insert(
    state: &Mutex<InMemoryState>,
    attempt: DesktopOAuthAttempt,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    anyhow::ensure!(
        state
            .oauth_states
            .iter()
            .any(|s| s.id == attempt.oauth_state_id),
        "missing desktop OAuth state"
    );
    anyhow::ensure!(
        !state
            .desktop_oauth_attempts
            .iter()
            .any(|s| s.attempt.id == attempt.id
                || s.attempt.oauth_state_id == attempt.oauth_state_id
                || s.attempt.secret_hash == attempt.secret_hash),
        "duplicate desktop OAuth attempt"
    );
    state
        .desktop_oauth_attempts
        .push(InMemoryDesktopOAuthAttempt {
            attempt,
            status: DesktopOAuthStatus::Pending,
            handoff_id: None,
            identity: None,
        });
    Ok(())
}

/// Находит попытку через явную связь со строкой OAuth state.
pub(super) fn find_by_state(
    state: &Mutex<InMemoryState>,
    state_hash: &str,
) -> anyhow::Result<Option<Uuid>> {
    let state = state.lock().map_err(|_| poisoned())?;
    let Some(oauth_state) = state
        .oauth_states
        .iter()
        .find(|s| s.state_hash == state_hash)
    else {
        return Ok(None);
    };
    Ok(state
        .desktop_oauth_attempts
        .iter()
        .find(|s| s.attempt.oauth_state_id == oauth_state.id)
        .map(|s| s.attempt.id))
}

/// Читает состояние без потребления, проверив секрет исходного приложения.
pub(super) fn status(
    state: &Mutex<InMemoryState>,
    id: &Uuid,
    secret_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<DesktopOAuthStatus>> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state
        .desktop_oauth_attempts
        .iter()
        .find(|s| s.attempt.id == *id && s.attempt.secret_hash == secret_hash)
        .map(|s| {
            if s.attempt.expires_at <= now
                && matches!(
                    s.status,
                    DesktopOAuthStatus::Pending | DesktopOAuthStatus::Ready
                )
            {
                DesktopOAuthStatus::Expired
            } else {
                s.status.clone()
            }
        }))
}

/// Проверяет возможность принять callback провайдера.
pub(super) fn is_pending(
    state: &Mutex<InMemoryState>,
    id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state.desktop_oauth_attempts.iter().any(|s| {
        s.attempt.id == *id && s.attempt.expires_at > now && s.status == DesktopOAuthStatus::Pending
    }))
}

/// Сохраняет личность и связанный handoff без мутаций аккаунта пользователя.
pub(super) fn finish(
    state: &Mutex<InMemoryState>,
    id: &Uuid,
    kind: String,
    user_id: Option<Uuid>,
    identity: DesktopOAuthIdentity,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let Some(row) = state.desktop_oauth_attempts.iter_mut().find(|s| {
        s.attempt.id == *id && s.attempt.expires_at > now && s.status == DesktopOAuthStatus::Pending
    }) else {
        return Ok(false);
    };
    let handoff = InMemoryOAuthHandoff {
        id: Uuid::new_v4(),
        code_hash: row.attempt.secret_hash.clone(),
        kind,
        user_id,
        registration_intent_id: None,
        expires_at: row.attempt.expires_at,
        consumed_at: None,
    };
    row.handoff_id = Some(handoff.id);
    row.identity = Some(identity);
    row.status = DesktopOAuthStatus::Ready;
    state.oauth_handoffs.push(handoff);
    Ok(true)
}

/// Читает личность готового handoff; изменение аккаунта требует отдельного атомарного захвата.
pub(super) fn identity_for_handoff(
    state: &Mutex<InMemoryState>,
    handoff_id: &Uuid,
) -> anyhow::Result<Option<DesktopOAuthIdentity>> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state
        .desktop_oauth_attempts
        .iter()
        .find(|s| {
            s.handoff_id == Some(*handoff_id)
                && matches!(
                    s.status,
                    DesktopOAuthStatus::Ready | DesktopOAuthStatus::Claimed
                )
        })
        .and_then(|s| s.identity.clone()))
}

/// Сохраняет ошибку только для действующей ожидающей попытки.
pub(super) fn fail(
    state: &Mutex<InMemoryState>,
    id: &Uuid,
    message: String,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let Some(row) = state.desktop_oauth_attempts.iter_mut().find(|s| {
        s.attempt.id == *id && s.attempt.expires_at > now && s.status == DesktopOAuthStatus::Pending
    }) else {
        return Ok(false);
    };
    row.status = DesktopOAuthStatus::Failed(message);
    Ok(true)
}

/// Отменяет попытку и потребляет её handoff под общей блокировкой.
pub(super) fn cancel(
    state: &Mutex<InMemoryState>,
    id: &Uuid,
    secret_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let Some(row) = state.desktop_oauth_attempts.iter_mut().find(|s| {
        s.attempt.id == *id
            && s.attempt.secret_hash == secret_hash
            && s.attempt.expires_at > now
            && matches!(
                s.status,
                DesktopOAuthStatus::Pending | DesktopOAuthStatus::Ready
            )
    }) else {
        return Ok(false);
    };
    row.status = DesktopOAuthStatus::Cancelled;
    let handoff_id = row.handoff_id;
    if let Some(handoff) = state
        .oauth_handoffs
        .iter_mut()
        .find(|h| Some(h.id) == handoff_id)
    {
        handoff.consumed_at = Some(now);
    }
    Ok(true)
}
