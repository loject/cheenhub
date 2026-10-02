//! Хранилище настроек хоста с недоступной историей активности.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::host_settings::domain::{
    GmailOAuthState, HostEmailSettings, VoiceActivitySample,
};
use crate::features::host_settings::infrastructure::HostSettingsStore;

/// Обёртка, у которой все операции истории завершаются ошибкой базы данных.
pub(super) struct UnavailableHostSettingsStore {
    inner: Arc<dyn HostSettingsStore>,
}

impl UnavailableHostSettingsStore {
    /// Оборачивает рабочее хранилище, сохраняя права владельца и настройки почты.
    pub(super) fn new(inner: Arc<dyn HostSettingsStore>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl HostSettingsStore for UnavailableHostSettingsStore {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool> {
        self.inner.is_host_owner(user_id).await
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
        _sample: VoiceActivitySample,
    ) -> anyhow::Result<()> {
        anyhow::bail!("host settings database is unavailable")
    }

    async fn load_voice_activity_samples(
        &self,
        _since: DateTime<Utc>,
        _now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<VoiceActivitySample>> {
        anyhow::bail!("host settings database is unavailable")
    }

    async fn delete_voice_activity_samples_before(
        &self,
        _cutoff: DateTime<Utc>,
    ) -> anyhow::Result<usize> {
        anyhow::bail!("host settings database is unavailable")
    }
}
