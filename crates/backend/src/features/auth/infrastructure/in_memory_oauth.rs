//! In-memory OAuth storage helpers.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState,
};
use crate::features::auth::infrastructure::in_memory::model::{
    InMemoryOAuthHandoff, InMemoryOAuthRegistrationIntent, InMemoryOAuthState, InMemoryState,
};

pub(super) fn insert_oauth_state(
    state: &Mutex<InMemoryState>,
    state_hash: String,
    nonce: String,
    flow_kind: String,
    user_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
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

pub(super) fn consume_oauth_state(
    state: &Mutex<InMemoryState>,
    state_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<OAuthState>> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
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

pub(super) fn find_oauth_account_by_subject(
    state: &Mutex<InMemoryState>,
    provider: &str,
    provider_subject: &str,
) -> anyhow::Result<Option<OAuthAccount>> {
    let state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    Ok(state
        .oauth_accounts
        .iter()
        .find(|account| {
            account.provider == provider && account.provider_subject == provider_subject
        })
        .cloned())
}

pub(super) fn find_oauth_account_for_user(
    state: &Mutex<InMemoryState>,
    provider: &str,
    user_id: &Uuid,
) -> anyhow::Result<Option<OAuthAccount>> {
    let state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    Ok(state
        .oauth_accounts
        .iter()
        .find(|account| account.provider == provider && account.user_id == *user_id)
        .cloned())
}

pub(super) fn list_oauth_accounts(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
) -> anyhow::Result<Vec<OAuthAccount>> {
    let state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    Ok(state
        .oauth_accounts
        .iter()
        .filter(|account| account.user_id == *user_id)
        .cloned()
        .collect())
}

pub(super) fn insert_oauth_account(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    provider: String,
    provider_subject: String,
    email: String,
    display_name: Option<String>,
    now: DateTime<Utc>,
) -> anyhow::Result<OAuthAccount> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    if state
        .oauth_accounts
        .iter()
        .any(|account| account.provider == provider && account.provider_subject == provider_subject)
    {
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

pub(super) fn delete_oauth_account(
    state: &Mutex<InMemoryState>,
    provider: &str,
    user_id: &Uuid,
) -> anyhow::Result<bool> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    let previous_len = state.oauth_accounts.len();
    state
        .oauth_accounts
        .retain(|account| account.provider != provider || account.user_id != *user_id);

    Ok(state.oauth_accounts.len() != previous_len)
}

pub(super) fn insert_oauth_handoff(
    state: &Mutex<InMemoryState>,
    code_hash: String,
    kind: String,
    user_id: Option<Uuid>,
    registration_intent_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
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

pub(super) fn find_active_oauth_handoff(
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

pub(super) fn consume_oauth_handoff(
    state: &Mutex<InMemoryState>,
    handoff_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    if let Some(handoff) = state
        .oauth_handoffs
        .iter_mut()
        .find(|handoff| handoff.id == *handoff_id)
    {
        handoff.consumed_at = Some(now);
    }

    Ok(())
}

pub(super) fn insert_oauth_registration_intent(
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

pub(super) fn find_active_oauth_registration_intent(
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

pub(super) fn consume_oauth_registration_intent(
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
