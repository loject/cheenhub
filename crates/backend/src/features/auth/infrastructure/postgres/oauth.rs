use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::PostgresAuthStore;
use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState,
};
use crate::features::auth::infrastructure::OAuthStore;

#[async_trait]
impl OAuthStore for PostgresAuthStore {
    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        super::super::postgres_oauth::insert_oauth_state(
            &self.database,
            state_hash,
            nonce,
            flow_kind,
            user_id,
            now,
            expires_at,
        )
        .await
    }

    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>> {
        super::super::postgres_oauth::consume_oauth_state(&self.database, state_hash, now).await
    }

    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::super::postgres_oauth::find_oauth_account_by_subject(
            &self.database,
            provider,
            provider_subject,
        )
        .await
    }

    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::super::postgres_oauth::find_oauth_account_for_user(&self.database, provider, user_id)
            .await
    }

    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>> {
        super::super::postgres_oauth::list_oauth_accounts(&self.database, user_id).await
    }

    async fn insert_oauth_account(
        &self,
        user_id: &Uuid,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<OAuthAccount> {
        super::super::postgres_oauth::insert_oauth_account(
            &self.database,
            user_id,
            provider,
            provider_subject,
            email,
            display_name,
            now,
        )
        .await
    }

    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool> {
        super::super::postgres_oauth::delete_oauth_account(&self.database, provider, user_id).await
    }

    async fn insert_oauth_handoff(
        &self,
        code_hash: String,
        kind: String,
        user_id: Option<Uuid>,
        registration_intent_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::super::postgres_oauth_handoff::insert(
            &self.database,
            code_hash,
            kind,
            user_id,
            registration_intent_id,
            now,
            expires_at,
        )
        .await
    }

    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>> {
        super::super::postgres_oauth_handoff::find_active(&self.database, code_hash, now).await
    }

    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        super::super::postgres_oauth_handoff::consume(&self.database, handoff_id, now).await
    }

    async fn insert_oauth_registration_intent(
        &self,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<OAuthRegistrationIntent> {
        super::super::postgres_oauth_handoff::insert_registration_intent(
            &self.database,
            provider,
            provider_subject,
            email,
            display_name,
            now,
            expires_at,
        )
        .await
    }

    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
        super::super::postgres_oauth_handoff::find_active_registration_intent(
            &self.database,
            intent_id,
            now,
        )
        .await
    }

    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::super::postgres_oauth_handoff::consume_registration_intent(
            &self.database,
            intent_id,
            now,
        )
        .await
    }
}
