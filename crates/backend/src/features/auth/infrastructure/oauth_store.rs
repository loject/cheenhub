use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState,
};

/// Граница OAuth-хранилища аутентификации.
#[async_trait]
pub(crate) trait OAuthStore: Send + Sync {
    /// Вставляет краткоживущий OAuth state.
    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid>;

    /// Потребляет активный OAuth state.
    async fn consume_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthState>>;

    /// Находит привязанный OAuth-аккаунт по subject провайдера.
    async fn find_oauth_account_by_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> anyhow::Result<Option<OAuthAccount>>;

    /// Находит привязанный OAuth-аккаунт для пользователя.
    async fn find_oauth_account_for_user(
        &self,
        provider: &str,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<OAuthAccount>>;

    /// Список привязанных OAuth-аккаунтов пользователя.
    async fn list_oauth_accounts(&self, user_id: &Uuid) -> anyhow::Result<Vec<OAuthAccount>>;

    /// Вставляет привязанный OAuth-аккаунт.
    async fn insert_oauth_account(
        &self,
        user_id: &Uuid,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<OAuthAccount>;

    /// Удаляет привязанный OAuth-аккаунт пользователя.
    async fn delete_oauth_account(&self, provider: &str, user_id: &Uuid) -> anyhow::Result<bool>;

    /// Вставляет краткоживущий OAuth-handoff для фронтенда.
    async fn insert_oauth_handoff(
        &self,
        code_hash: String,
        kind: String,
        user_id: Option<Uuid>,
        registration_intent_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Находит активный OAuth-handoff фронтенда.
    async fn find_active_oauth_handoff(
        &self,
        code_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthHandoff>>;

    /// Атомарно помечает активный OAuth-handoff фронтенда как использованный.
    ///
    /// Возвращает `true`, только если текущий вызов первым потребил handoff.
    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Вставляет краткоживущее намерение регистрации OAuth.
    async fn insert_oauth_registration_intent(
        &self,
        provider: String,
        provider_subject: String,
        email: String,
        display_name: Option<String>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<OAuthRegistrationIntent>;

    /// Находит активное намерение регистрации OAuth.
    async fn find_active_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<OAuthRegistrationIntent>>;

    /// Помечает намерение регистрации OAuth как использованное.
    async fn consume_oauth_registration_intent(
        &self,
        intent_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;
}
