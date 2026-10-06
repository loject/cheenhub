//! In-memory хранилище глобальных настроек хоста.
//!
//! Реализация нужна только для локальной разработки и тестов, поэтому
//! хранит данные в обычных блокировках без индексов, кэшей и очистки.

use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::super::domain::{
    GmailOAuthState, HostEmailSettings, HostLogSettings, HostOwner, RevokeHostOwnerOutcome,
    VoiceActivitySample,
};
use super::HostSettingsStore;

/// Хранилище настроек хоста в памяти процесса для разработки и тестов.
#[derive(Default)]
pub(crate) struct InMemoryHostSettingsStore {
    settings: RwLock<HostEmailSettings>,
    log_settings: RwLock<HostLogSettings>,
    owners: RwLock<Vec<HostOwner>>,
    states: RwLock<Vec<(GmailOAuthState, Option<DateTime<Utc>>)>>,
    pub(super) voice_activity: RwLock<Vec<VoiceActivitySample>>,
}

impl InMemoryHostSettingsStore {
    #[cfg(test)]
    pub(crate) fn with_owner(user_id: Uuid) -> Self {
        Self {
            owners: RwLock::new(vec![HostOwner {
                user_id,
                granted_at: Utc::now(),
                granted_by_user_id: None,
            }]),
            ..Self::default()
        }
    }
}

#[async_trait]
impl HostSettingsStore for InMemoryHostSettingsStore {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool> {
        Ok(self
            .owners
            .read()
            .expect("host owners lock")
            .iter()
            .any(|owner| owner.user_id == user_id))
    }

    async fn load_host_owners(&self) -> anyhow::Result<Vec<HostOwner>> {
        Ok(self.owners.read().expect("host owners lock").clone())
    }

    async fn grant_host_owner(&self, owner: HostOwner) -> anyhow::Result<()> {
        let mut owners = self.owners.write().expect("host owners lock");
        match owners
            .iter_mut()
            .find(|existing| existing.user_id == owner.user_id)
        {
            Some(existing) => *existing = owner,
            None => owners.push(owner),
        }
        Ok(())
    }

    async fn revoke_host_owner(&self, user_id: Uuid) -> anyhow::Result<RevokeHostOwnerOutcome> {
        let mut owners = self.owners.write().expect("host owners lock");
        if !owners.iter().any(|owner| owner.user_id == user_id) {
            return Ok(RevokeHostOwnerOutcome::Missing);
        }
        if owners.len() == 1 {
            return Ok(RevokeHostOwnerOutcome::LastOwner);
        }
        owners.retain(|owner| owner.user_id != user_id);
        Ok(RevokeHostOwnerOutcome::Revoked)
    }

    async fn load_email_settings(&self) -> anyhow::Result<HostEmailSettings> {
        Ok(self.settings.read().expect("host settings lock").clone())
    }

    async fn save_email_settings(
        &self,
        settings: HostEmailSettings,
        _updated_by: Uuid,
        _updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostEmailSettings> {
        *self.settings.write().expect("host settings lock") = settings.clone();
        Ok(settings)
    }

    async fn load_log_settings(&self) -> anyhow::Result<HostLogSettings> {
        Ok(self
            .log_settings
            .read()
            .expect("host log settings lock")
            .clone())
    }

    async fn save_log_settings(
        &self,
        settings: HostLogSettings,
        _updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostLogSettings> {
        let mut stored = settings;
        stored.updated_at = Some(updated_at);
        *self.log_settings.write().expect("host log settings lock") = stored.clone();
        Ok(stored)
    }

    async fn insert_gmail_oauth_state(&self, state: GmailOAuthState) -> anyhow::Result<()> {
        self.states
            .write()
            .expect("oauth states lock")
            .push((state, None));
        Ok(())
    }

    async fn consume_gmail_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>> {
        let mut states = self.states.write().expect("oauth states lock");
        let Some((state, consumed_at)) = states.iter_mut().find(|(state, consumed_at)| {
            state.state_hash == state_hash && consumed_at.is_none() && state.expires_at > now
        }) else {
            return Ok(None);
        };
        *consumed_at = Some(now);
        Ok(Some(state.user_id))
    }

    async fn insert_voice_activity_sample(
        &self,
        sample: VoiceActivitySample,
    ) -> anyhow::Result<()> {
        self.voice_activity
            .write()
            .expect("voice activity lock")
            .push(sample);
        Ok(())
    }

    async fn load_voice_activity_samples(
        &self,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<VoiceActivitySample>> {
        let samples = self.voice_activity.read().expect("voice activity lock");
        let mut selected: Vec<VoiceActivitySample> = samples
            .iter()
            .copied()
            .filter(|sample| sample.sampled_at > since && sample.sampled_at <= now)
            .collect();
        selected.sort_by_key(|sample| sample.sampled_at);
        Ok(selected)
    }

    async fn delete_voice_activity_samples_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> anyhow::Result<usize> {
        let mut samples = self.voice_activity.write().expect("voice activity lock");
        let before = samples.len();
        samples.retain(|sample| sample.sampled_at >= cutoff);
        Ok(before - samples.len())
    }
}
