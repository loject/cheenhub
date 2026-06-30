//! Слой инфраструктуры аутентификации.

mod conversions;
mod entities;
mod in_memory;
mod in_memory_oauth;
mod in_memory_password_reset;
mod in_memory_profile;
mod in_memory_refresh;
mod postgres;
mod postgres_oauth;
mod postgres_password_reset;
mod postgres_profile;
mod postgres_refresh;
mod postgres_user;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState, PasswordResetToken,
    RefreshSession, UserAccount, UserSession,
};

pub(crate) use in_memory::InMemoryAuthStore;
pub(crate) use postgres::PostgresAuthStore;

/// Конфликт уникального поля пользователя.
#[derive(Debug)]
pub(crate) enum UserConflict {
    /// Конфликт никнейма.
    Nickname,
    /// Конфликт email.
    Email,
}

/// Ошибка, возвращаемая при вставке пользователя.
#[derive(Debug)]
pub(crate) enum InsertUserError {
    /// Конфликт уникального поля.
    Conflict(UserConflict),
    /// Непредвиденная ошибка базы данных.
    Database(sea_orm::DbErr),
    /// Непредвиденная ошибка хранилища.
    Storage(anyhow::Error),
}

/// Ошибка, возвращаемая при обновлении никнейма пользователя.
#[derive(Debug)]
pub(crate) enum UpdateUserNicknameError {
    /// Конфликт уникального поля.
    Conflict(UserConflict),
    /// Nickname was changed too recently.
    Cooldown {
        /// First timestamp when another nickname change is allowed.
        next_allowed_at: DateTime<Utc>,
    },
    /// Непредвиденная ошибка базы данных.
    Database(sea_orm::DbErr),
    /// Unexpected storage error.
    Storage(anyhow::Error),
}

/// Граница хранилища аутентификации.
#[async_trait]
pub(crate) trait AuthStore: Send + Sync {
    /// Вставляет новую учетную запись пользователя.
    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<UserAccount, InsertUserError>;

    /// Находит пользователя по нормализованному email.
    async fn find_user_by_email(
        &self,
        email_normalized: &str,
    ) -> anyhow::Result<Option<UserAccount>>;

    /// Находит пользователя по идентификатору.
    async fn find_user_by_id(&self, user_id: &Uuid) -> anyhow::Result<Option<UserAccount>>;

    /// Ищет пользователей по части никнейма.
    async fn search_users_by_nickname(
        &self,
        query: &str,
        limit: u64,
    ) -> anyhow::Result<Vec<UserAccount>>;

    /// Обновляет публичный никнейм пользователя.
    async fn update_user_nickname(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        nickname: String,
        now: DateTime<Utc>,
        cooldown: Duration,
    ) -> Result<Option<UserAccount>, UpdateUserNicknameError>;

    /// Обновляет текущий идентификатор изображения аватара пользователя.
    async fn update_user_avatar_image_id(
        &self,
        user_id: &Uuid,
        image_id: Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<UserAccount>>;

    /// Находит текущие идентификаторы изображений аватаров пользователей.
    async fn avatar_image_ids_by_user_ids(
        &self,
        user_ids: &[Uuid],
    ) -> anyhow::Result<HashMap<Uuid, Uuid>>;

    /// Обновляет хеш пароля пользователя.
    async fn update_user_password_hash(
        &self,
        user_id: &Uuid,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Обновляет хеш пароля пользователя и записывает трассировку смены пароля профиля.
    async fn change_user_password(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        password_hash: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Создает сессию и ее начальную строку refresh-токена.
    async fn create_session(
        &self,
        user_id: &Uuid,
        refresh_hash: String,
        user_agent: Option<&str>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<Uuid>;

    /// Находит активную refresh-сессию по хешу токена.
    async fn find_active_refresh(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<RefreshSession>>;

    /// Выполняет ротацию refresh-токена для существующей сессии.
    async fn rotate_refresh(
        &self,
        old_refresh_id: &Uuid,
        session_id: &Uuid,
        next_hash: String,
        user_agent: Option<&str>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Отзывает refresh-токен и принадлежащую ему сессию.
    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Обнаруживает повторное использование уже ротированного/отозванного
    /// refresh-токена и в этом случае принудительно отзывает всю сессию.
    ///
    /// Это стандартная реакция на кражу refresh-токена (RFC 6819): если кто-то
    /// предъявляет токен, который уже был ротирован, вся цепочка сессии
    /// аннулируется, и легитимному пользователю придется войти заново.
    /// Возвращает `true`, если повторное использование обнаружено и сессия отозвана.
    async fn revoke_session_on_refresh_reuse(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Возвращает, активна ли сессия сейчас.
    async fn session_is_active(
        &self,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Возвращает активные сессии, принадлежащие пользователю.
    async fn list_active_sessions(
        &self,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<UserSession>>;

    /// Отзывает одну активную сессию пользователя.
    async fn revoke_user_session(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Отзывает все активные сессии пользователя.
    async fn revoke_user_sessions(&self, user_id: &Uuid, now: DateTime<Utc>) -> anyhow::Result<()>;

    /// Вставляет краткоживущий токен сброса пароля.
    async fn insert_password_reset_token(
        &self,
        user_id: &Uuid,
        token_hash: String,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()>;

    /// Потребляет активный токен сброса пароля.
    async fn consume_password_reset_token(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<PasswordResetToken>>;

    /// Вставляет краткоживущий OAuth state.
    async fn insert_oauth_state(
        &self,
        state_hash: String,
        nonce: String,
        flow_kind: String,
        user_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<()>;

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

    /// Помечает OAuth-handoff фронтенда как использованный.
    async fn consume_oauth_handoff(
        &self,
        handoff_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;

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
