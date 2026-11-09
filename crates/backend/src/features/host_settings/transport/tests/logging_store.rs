//! Воспроизводит лог SQLx внутри чтения прав без внешней базы данных.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::host_settings::domain::{
    GmailOAuthState, HostEmailSettings, HostLogSettings, HostOwner, HostRegistrationSettings,
    RevokeHostOwnerOutcome, VoiceActivitySample,
};
use crate::features::host_settings::infrastructure::HostSettingsStore;

/// Делегирует хранение, добавляя наблюдаемый лог при проверке владельца.
pub(super) struct LoggingStore(
    /// Рабочее хранилище, определяющее результат и изменения прав.
    pub(super) Arc<dyn HostSettingsStore>,
);

#[async_trait]
impl HostSettingsStore for LoggingStore {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool> {
        let result = self.0.is_host_owner(user_id).await;
        tracing::info!(target: "sqlx::query", summary = "SELECT host_owners", "owner access query");
        result
    }

    async fn load_host_owners(&self) -> anyhow::Result<Vec<HostOwner>> {
        self.0.load_host_owners().await
    }

    async fn grant_host_owner(&self, owner: HostOwner) -> anyhow::Result<()> {
        self.0.grant_host_owner(owner).await
    }

    async fn revoke_host_owner(&self, user_id: Uuid) -> anyhow::Result<RevokeHostOwnerOutcome> {
        self.0.revoke_host_owner(user_id).await
    }

    async fn load_email_settings(&self) -> anyhow::Result<HostEmailSettings> {
        self.0.load_email_settings().await
    }

    async fn save_email_settings(
        &self,
        settings: HostEmailSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostEmailSettings> {
        self.0
            .save_email_settings(settings, updated_by, updated_at)
            .await
    }

    async fn load_log_settings(&self) -> anyhow::Result<HostLogSettings> {
        self.0.load_log_settings().await
    }

    async fn save_log_settings(
        &self,
        settings: HostLogSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostLogSettings> {
        self.0
            .save_log_settings(settings, updated_by, updated_at)
            .await
    }

    async fn load_registration_settings(&self) -> anyhow::Result<HostRegistrationSettings> {
        self.0.load_registration_settings().await
    }

    async fn save_registration_settings(
        &self,
        settings: HostRegistrationSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostRegistrationSettings> {
        self.0
            .save_registration_settings(settings, updated_by, updated_at)
            .await
    }

    async fn insert_gmail_oauth_state(&self, state: GmailOAuthState) -> anyhow::Result<()> {
        self.0.insert_gmail_oauth_state(state).await
    }

    async fn consume_gmail_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>> {
        self.0.consume_gmail_oauth_state(state_hash, now).await
    }

    async fn insert_voice_activity_sample(
        &self,
        sample: VoiceActivitySample,
    ) -> anyhow::Result<()> {
        self.0.insert_voice_activity_sample(sample).await
    }

    async fn load_voice_activity_samples(
        &self,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<VoiceActivitySample>> {
        self.0.load_voice_activity_samples(since, now).await
    }

    async fn delete_voice_activity_samples_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> anyhow::Result<usize> {
        self.0.delete_voice_activity_samples_before(cutoff).await
    }
}
