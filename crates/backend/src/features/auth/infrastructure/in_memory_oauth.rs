//! In-memory OAuth storage helpers.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{OAuthAccount, OAuthState};
use crate::features::auth::infrastructure::in_memory::model::{InMemoryOAuthState, InMemoryState};

pub(super) fn insert_oauth_state(
    state: &Mutex<InMemoryState>,
    state_hash: String,
    nonce: String,
    flow_kind: String,
    user_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<Uuid> {
    let mut state = state.lock().map_err(|_| super::in_memory::poisoned())?;
    anyhow::ensure!(
        !user_id.is_some_and(|id| state
            .users
            .iter()
            .any(|user| user.account.id == id && user.deletion.is_some())),
        "account is deleted"
    );
    let id = Uuid::new_v4();
    state.oauth_states.push(InMemoryOAuthState {
        id,
        state_hash,
        nonce,
        flow_kind,
        user_id,
        expires_at,
        consumed_at: None,
    });

    Ok(id)
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
        id: oauth_state.id,
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
    anyhow::ensure!(
        !state
            .users
            .iter()
            .any(|user| user.account.id == *user_id && user.deletion.is_some()),
        "account is deleted"
    );
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
