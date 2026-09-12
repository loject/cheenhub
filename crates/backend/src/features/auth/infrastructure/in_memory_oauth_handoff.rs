//! In-memory операции OAuth-передачи и регистрации.

use crate::features::auth::domain::{DesktopOAuthStatus, OAuthHandoff, OAuthRegistrationIntent};
use crate::features::auth::infrastructure::in_memory::model::{
    InMemoryOAuthHandoff, InMemoryOAuthRegistrationIntent, InMemoryState,
};
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use uuid::Uuid;

pub(super) fn insert(
    state: &Mutex<InMemoryState>,
    code_hash: String,
    kind: String,
    user_id: Option<Uuid>,
    registration_intent_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    anyhow::ensure!(
        !user_id.is_some_and(|id| state
            .users
            .iter()
            .any(|user| user.account.id == id && user.deletion.is_some())),
        "account is deleted"
    );
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
pub(super) fn find_active(
    state: &Mutex<InMemoryState>,
    code_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<OAuthHandoff>> {
    let state = state.lock().map_err(|_| super::in_memory::poisoned())?;
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
pub(super) fn consume(
    state: &Mutex<InMemoryState>,
    handoff_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    if let Some(attempt) = state
        .desktop_oauth_attempts
        .iter()
        .find(|attempt| attempt.handoff_id == Some(*handoff_id))
        && (attempt.status != DesktopOAuthStatus::Ready || attempt.attempt.expires_at <= now)
    {
        return Ok(false);
    }
    let Some(handoff) = state.oauth_handoffs.iter_mut().find(|handoff| {
        handoff.id == *handoff_id && handoff.consumed_at.is_none() && handoff.expires_at > now
    }) else {
        return Ok(false);
    };
    handoff.consumed_at = Some(now);
    if let Some(attempt) = state
        .desktop_oauth_attempts
        .iter_mut()
        .find(|attempt| attempt.handoff_id == Some(*handoff_id))
    {
        attempt.status = DesktopOAuthStatus::Claimed;
    }
    Ok(true)
}
pub(super) fn insert_registration_intent(
    state: &Mutex<InMemoryState>,
    provider: String,
    provider_subject: String,
    email: String,
    display_name: Option<String>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<OAuthRegistrationIntent> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
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
pub(super) fn find_active_registration_intent(
    state: &Mutex<InMemoryState>,
    intent_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
    let state = state.lock().map_err(|_| super::in_memory::poisoned())?;
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
pub(super) fn consume_registration_intent(
    state: &Mutex<InMemoryState>,
    intent_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    if let Some(intent) = state
        .oauth_registration_intents
        .iter_mut()
        .find(|intent| intent.intent.id == *intent_id)
    {
        intent.consumed_at = Some(now);
    }
    Ok(())
}
