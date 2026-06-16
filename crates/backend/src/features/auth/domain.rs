//! Модели домена аутентификации.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Данные учетной записи пользователя, используемые в потоках аутентификации.
#[derive(Debug, Clone)]
pub(crate) struct UserAccount {
    /// Стабильный идентификатор пользователя.
    pub(crate) id: Uuid,
    /// Публичный никнейм, отображаемый другим пользователям.
    pub(crate) nickname: String,
    /// Адрес электронной почты, используемый для входа.
    pub(crate) email: String,
    /// Сохраненный хеш пароля Argon2.
    pub(crate) password_hash: Option<String>,
    /// Текущий идентификатор изображения аватара.
    pub(crate) avatar_image_id: Option<Uuid>,
    /// Метка времени регистрации учетной записи.
    pub(crate) registered_at: DateTime<Utc>,
    /// Метка времени последнего успешного обновления никнейма.
    pub(crate) nickname_updated_at: DateTime<Utc>,
}

/// Активная сессия refresh-токена с владельцем.
#[derive(Debug, Clone)]
pub(crate) struct RefreshSession {
    /// Идентификатор строки refresh-токена.
    pub(crate) refresh_token_id: Uuid,
    /// Идентификатор строки сессии.
    pub(crate) session_id: Uuid,
    /// User that owns the session.
    pub(crate) user: UserAccount,
}

/// Активные данные сессии аутентификации, используемые настройками безопасности учетной записи.
#[derive(Debug, Clone)]
pub(crate) struct UserSession {
    /// Идентификатор строки сессии.
    pub(crate) id: Uuid,
    /// Метка времени создания сессии.
    pub(crate) created_at: DateTime<Utc>,
    /// Метка времени последней наблюдаемой активности.
    pub(crate) last_seen_at: DateTime<Utc>,
    /// Метка времени истечения сессии.
    pub(crate) expires_at: DateTime<Utc>,
    /// Недавно наблюдаемый нормализованный User-Agent для сессии.
    pub(crate) user_agent: Option<String>,
}

/// Данные связанной учетной записи OAuth.
#[derive(Debug, Clone)]
pub(crate) struct OAuthAccount {
    /// Пользователь, владеющий связанной учетной записью.
    pub(crate) user_id: Uuid,
    /// Внешний провайдер OAuth.
    pub(crate) provider: String,
    /// Стабильный идентификатор на стороне провайдера.
    pub(crate) provider_subject: String,
    /// Provider email address.
    pub(crate) email: String,
    /// Отображаемое имя провайдера.
    pub(crate) display_name: Option<String>,
    /// Метка времени, когда провайдер был связан.
    pub(crate) linked_at: DateTime<Utc>,
}

/// Одноразовое состояние OAuth, созданное перед перенаправлением к провайдеру.
#[derive(Debug, Clone)]
pub(crate) struct OAuthState {
    /// OAuth нонс, отправляемый провайдеру.
    pub(crate) nonce: String,
    /// Вид потока.
    pub(crate) flow_kind: String,
    /// Аутентифицированный пользователь для потока привязки.
    pub(crate) user_id: Option<Uuid>,
}

/// Краткоживущее намерение регистрации OAuth.
#[derive(Debug, Clone)]
pub(crate) struct OAuthRegistrationIntent {
    /// Стабильный идентификатор строки намерения.
    pub(crate) id: Uuid,
    /// Стабильный идентификатор на стороне провайдера.
    pub(crate) provider_subject: String,
    /// Проверенный адрес электронной почты провайдера.
    pub(crate) email: String,
    /// Отображаемое имя провайдера.
    pub(crate) display_name: Option<String>,
}

/// Одноразовый OAuth-handoff для фронтенда.
#[derive(Debug, Clone)]
pub(crate) struct OAuthHandoff {
    /// Стабильный идентификатор строки handoff.
    pub(crate) id: Uuid,
    /// Вид результата handoff.
    pub(crate) kind: String,
    /// ID пользователя для аутентифицированных и связанных handoffs.
    pub(crate) user_id: Option<Uuid>,
    /// ID намерения регистрации для регистрационных handoffs.
    pub(crate) registration_intent_id: Option<Uuid>,
}

/// Данные одноразового токена сброса пароля.
#[derive(Debug, Clone)]
pub(crate) struct PasswordResetToken {
    /// Стабильный идентификатор строки токена сброса пароля.
    pub(crate) id: Uuid,
    /// Пользователь, владеющий токеном сброса.
    pub(crate) user_id: Uuid,
}
