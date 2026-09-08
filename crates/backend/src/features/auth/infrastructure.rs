//! Слой инфраструктуры аутентификации.

mod account_lifecycle;
mod conversions;
#[cfg(test)]
mod deletion_tests;
#[cfg(test)]
mod desktop_oauth_postgres_tests;
mod entities;
mod in_memory;
mod in_memory_deletion;
mod in_memory_desktop_oauth;
mod in_memory_oauth;
mod in_memory_password_reset;
mod in_memory_profile;
mod in_memory_refresh;
mod in_memory_user;
mod postgres;
mod postgres_deletion;
mod postgres_deletion_finalize;
mod postgres_desktop_oauth;
mod postgres_desktop_oauth_cleanup;
mod postgres_oauth;
mod postgres_password_reset;
mod postgres_profile;
mod postgres_refresh;
mod postgres_store;
mod postgres_user;

#[cfg(test)]
mod desktop_oauth_tests;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::features::auth::domain::{
    AccountDeletion, DesktopOAuthAttempt, DesktopOAuthIdentity, DesktopOAuthStatus, OAuthAccount,
    OAuthHandoff, OAuthRegistrationIntent, OAuthState, PasswordResetToken, RefreshSession,
    RegistrationLegalAcceptance, UserAccount, UserSession,
};

pub(crate) use account_lifecycle::AccountLifecycleGuard;
pub(crate) use in_memory::InMemoryAuthStore;
pub(crate) use postgres_store::PostgresAuthStore;

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

/// Результат атомарной попытки ротации refresh-токена.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RotateRefreshOutcome {
    /// Старый токен потреблён, новый токен создан.
    Rotated,
    /// Старый токен уже потреблён, отозван или больше не принадлежит активной сессии.
    AlreadyConsumed,
}

/// Результат проверки неактивного refresh-токена на повторное использование.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefreshReuseOutcome {
    /// Токен неизвестен либо не является ранее ротированным токеном.
    NotDetected,
    /// Токен был ротирован совсем недавно конкурентным запросом; сессия сохранена.
    ConcurrentRotation,
    /// Токен отозван обычным завершением сессии и не является доказательством reuse.
    SessionRevoked,
    /// Токен повторно использован за пределами защитного окна; вся сессия отозвана.
    ReusedAndRevoked {
        /// Отозванная auth-сессия, realtime-транспорты которой нужно завершить.
        session_id: Uuid,
    },
}

/// Граница хранилища аутентификации.
#[async_trait]
pub(crate) trait AuthStore: Send + Sync {
    /// Сериализует прикладные операции удаления аккаунта и создания серверов.
    async fn lock_account_lifecycle(&self, user_id: &Uuid)
    -> anyhow::Result<AccountLifecycleGuard>;

    /// Возвращает tombstone удалённого аккаунта.
    async fn account_deletion(&self, user_id: &Uuid) -> anyhow::Result<Option<AccountDeletion>>;

    /// Атомарно создаёт tombstone и отзывает доступ пользователя.
    async fn begin_account_deletion(
        &self,
        user_id: &Uuid,
        token_hash: String,
        now: DateTime<Utc>,
        restore_until: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Однократно восстанавливает аккаунт до окончания срока.
    async fn restore_account(&self, token_hash: &str, now: DateTime<Utc>) -> anyhow::Result<bool>;

    /// Обезличивает просроченные аккаунты, сохраняя UUID и tombstone.
    async fn finalize_expired_account_deletions(&self, now: DateTime<Utc>) -> anyhow::Result<u64>;

    /// Создаёт desktop-попытку со ссылкой на OAuth state и хешем секрета.
    async fn insert_desktop_oauth_attempt(
        &self,
        attempt: DesktopOAuthAttempt,
    ) -> anyhow::Result<()>;

    /// Находит desktop-попытку через связанную строку OAuth state, включая завершённые попытки.
    async fn desktop_oauth_attempt_by_state_hash(
        &self,
        state_hash: &str,
    ) -> anyhow::Result<Option<Uuid>>;

    /// Читает состояние по секрету без потребления готового результата.
    async fn desktop_oauth_status(
        &self,
        id: &Uuid,
        secret_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<DesktopOAuthStatus>>;

    /// Проверяет, ожидает ли действующая попытка ответ провайдера.
    async fn desktop_oauth_attempt_is_pending(
        &self,
        id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Атомарно сохраняет проверенную личность и создаёт связанный одноразовый handoff.
    async fn finish_desktop_oauth_attempt(
        &self,
        id: &Uuid,
        kind: String,
        user_id: Option<Uuid>,
        identity: DesktopOAuthIdentity,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Читает личность готового desktop-handoff; сам по себе вызов не разрешает вход.
    async fn desktop_oauth_identity_for_handoff(
        &self,
        handoff_id: &Uuid,
    ) -> anyhow::Result<Option<DesktopOAuthIdentity>>;

    /// Завершает ожидающую попытку ошибкой без создания сессии.
    async fn fail_desktop_oauth_attempt(
        &self,
        id: &Uuid,
        message: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Атомарно отменяет ожидающую или готовую попытку и отзывает её handoff.
    async fn cancel_desktop_oauth_attempt(
        &self,
        id: &Uuid,
        secret_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;

    /// Вставляет новую учетную запись пользователя.
    async fn insert_user(
        &self,
        nickname: String,
        email: String,
        email_normalized: String,
        password_hash: Option<String>,
        legal_acceptance: RegistrationLegalAcceptance,
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
    ) -> anyhow::Result<RotateRefreshOutcome>;

    /// Отзывает refresh-токен и принадлежащую ему сессию.
    ///
    /// Возвращает идентификатор найденной auth-сессии, чтобы вызывающий код
    /// мог завершить связанные с ней realtime-транспорты.
    async fn revoke_refresh_session(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>>;

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
        concurrent_rotation_after: DateTime<Utc>,
    ) -> anyhow::Result<RefreshReuseOutcome>;

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

    /// Записывает наблюдаемый User-Agent текущей auth-сессии.
    async fn record_session_user_agent(
        &self,
        session_id: &Uuid,
        user_agent: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;

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

    /// Находит активный токен сброса перед затратным хешированием нового пароля.
    async fn find_active_password_reset_token(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<PasswordResetToken>>;

    /// Атомарно потребляет токен, меняет пароль и отзывает остальные reset-токены пользователя.
    async fn complete_password_reset(
        &self,
        token_hash: &str,
        password_hash: String,
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
