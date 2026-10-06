//! Хранилище с управляемым сбоем или задержкой записи уровня журнала.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::host_settings::domain::{
    GmailOAuthState, HostEmailSettings, HostLogSettings, HostOwner, RevokeHostOwnerOutcome,
    VoiceActivitySample,
};
use crate::features::host_settings::infrastructure::HostSettingsStore;

/// Тестовая обёртка для воспроизведения сбоя и конкурирующих записей.
pub(super) struct ControlledStore {
    inner: Arc<dyn HostSettingsStore>,
    fail: bool,
    pub(super) started: tokio::sync::Notify,
    pub(super) release: tokio::sync::Notify,
    pub(super) finished: tokio::sync::Notify,
    saves: std::sync::atomic::AtomicUsize,
}

impl ControlledStore {
    /// Сохраняет остальные операции рабочего хранилища.
    pub(super) fn new(inner: Arc<dyn HostSettingsStore>, fail: bool) -> Self {
        Self {
            inner,
            fail,
            started: Default::default(),
            release: Default::default(),
            finished: Default::default(),
            saves: Default::default(),
        }
    }
}

#[async_trait]
impl HostSettingsStore for ControlledStore {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool> {
        self.inner.is_host_owner(user_id).await
    }

    async fn load_host_owners(&self) -> anyhow::Result<Vec<HostOwner>> {
        self.inner.load_host_owners().await
    }

    async fn grant_host_owner(&self, owner: HostOwner) -> anyhow::Result<()> {
        self.inner.grant_host_owner(owner).await
    }

    async fn revoke_host_owner(&self, user_id: Uuid) -> anyhow::Result<RevokeHostOwnerOutcome> {
        self.inner.revoke_host_owner(user_id).await
    }

    async fn load_email_settings(&self) -> anyhow::Result<HostEmailSettings> {
        self.inner.load_email_settings().await
    }

    async fn save_email_settings(
        &self,
        settings: HostEmailSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostEmailSettings> {
        self.inner
            .save_email_settings(settings, updated_by, updated_at)
            .await
    }

    async fn load_log_settings(&self) -> anyhow::Result<HostLogSettings> {
        self.inner.load_log_settings().await
    }

    async fn save_log_settings(
        &self,
        settings: HostLogSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostLogSettings> {
        if self.fail {
            anyhow::bail!("host settings database is unavailable");
        }
        if self.saves.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            self.started.notify_one();
            self.release.notified().await;
        }
        let result = self
            .inner
            .save_log_settings(settings, updated_by, updated_at)
            .await;
        self.finished.notify_one();
        result
    }

    async fn insert_gmail_oauth_state(&self, state: GmailOAuthState) -> anyhow::Result<()> {
        self.inner.insert_gmail_oauth_state(state).await
    }

    async fn consume_gmail_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>> {
        self.inner.consume_gmail_oauth_state(state_hash, now).await
    }

    async fn insert_voice_activity_sample(
        &self,
        sample: VoiceActivitySample,
    ) -> anyhow::Result<()> {
        self.inner.insert_voice_activity_sample(sample).await
    }

    async fn load_voice_activity_samples(
        &self,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<VoiceActivitySample>> {
        self.inner.load_voice_activity_samples(since, now).await
    }

    async fn delete_voice_activity_samples_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> anyhow::Result<usize> {
        self.inner
            .delete_voice_activity_samples_before(cutoff)
            .await
    }
}
