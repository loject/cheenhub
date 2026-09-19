use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::InMemoryAuthStore;
use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState,
};
use crate::features::auth::infrastructure::OAuthStore;

#[async_trait]
impl OAuthStore for InMemoryAuthStore {
    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        super::super::in_memory_oauth::insert_oauth_state(
            &self.state,
            state_hash,
            nonce,
            flow_kind,
            user_id,
            expires_at,
        )
    }

    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>> {
        super::super::in_memory_oauth::consume_oauth_state(&self.state, state_hash, now)
    }

    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::super::in_memory_oauth::find_oauth_account_by_subject(
            &self.state,
            provider,
            provider_subject,
        )
    }

    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>> {
        super::super::in_memory_oauth::find_oauth_account_for_user(&self.state, provider, user_id)
    }

    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>> {
        super::super::in_memory_oauth::list_oauth_accounts(&self.state, user_id)
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
        super::super::in_memory_oauth::insert_oauth_account(
            &self.state,
            user_id,
            provider,
            provider_subject,
            email,
            display_name,
            now,
        )
    }

    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool> {
        super::super::in_memory_oauth::delete_oauth_account(&self.state, provider, user_id)
    }

    async fn insert_oauth_handoff(
        &self,
        code_hash: String,
        kind: String,
        user_id: Option<Uuid>,
        registration_intent_id: Option<Uuid>,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::super::in_memory_oauth_handoff::insert(
            &self.state,
            code_hash,
            kind,
            user_id,
            registration_intent_id,
            expires_at,
        )
    }

    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>> {
        super::super::in_memory_oauth_handoff::find_active(&self.state, code_hash, now)
    }

    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        super::super::in_memory_oauth_handoff::consume(&self.state, handoff_id, now)
    }

    async fn insert_oauth_registration_intent(
        &self,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        _now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<OAuthRegistrationIntent> {
        super::super::in_memory_oauth_handoff::insert_registration_intent(
            &self.state,
            provider,
            provider_subject,
            email,
            display_name,
            expires_at,
        )
    }

    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
        super::super::in_memory_oauth_handoff::find_active_registration_intent(
            &self.state,
            intent_id,
            now,
        )
    }

    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        super::super::in_memory_oauth_handoff::consume_registration_intent(
            &self.state,
            intent_id,
            now,
        )
    }
}
