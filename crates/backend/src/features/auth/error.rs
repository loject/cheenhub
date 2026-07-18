//! Типы ошибок аутентификации.

/// Ошибка, возвращаемая потоками приложения аутентификации.
#[derive(Debug)]
pub(crate) enum AuthError {
    /// Данные запроса невалидны.
    BadRequest(String),
    /// Учетные данные или токены невалидны.
    Unauthorized(String),
    /// Refresh-токен подтверждённо больше не может продолжать сессию.
    RefreshRejected {
        /// Стабильная причина отказа для клиентской классификации.
        reason: RefreshRejection,
        /// Сообщение для пользователя.
        message: String,
    },
    /// Refresh-токен уже ротируется конкурентным запросом; клиенту следует дождаться новых токенов.
    RefreshRotationInProgress(String),
    /// Уникальное поле учетной записи уже существует.
    Conflict(String),
    /// Запрос валиден, но в данный момент ограничен частотой запросов.
    RateLimited(String),
    /// Требуемая интеграция времени выполнения не настроена.
    Misconfigured {
        /// Название функции или интеграции.
        feature: &'static str,
        /// Имена отсутствующих переменных окружения.
        missing: Vec<&'static str>,
        /// Сообщение для пользователя.
        message: String,
    },
    /// Непредвиденный сбой инфраструктуры.
    Internal(anyhow::Error),
}

impl AuthError {
    /// Возвращает сообщение для пользователя, когда эту ошибку можно безопасно показывать.
    pub(crate) fn user_message(&self) -> Option<&str> {
        match self {
            Self::BadRequest(message)
            | Self::Unauthorized(message)
            | Self::Conflict(message)
            | Self::RateLimited(message) => Some(message),
            Self::Misconfigured { message, .. } => Some(message),
            Self::RefreshRejected { message, .. } => Some(message),
            Self::RefreshRotationInProgress(message) => Some(message),
            Self::Internal(_) => None,
        }
    }
}

/// Подтверждённая сервером причина отказа refresh-токена.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefreshRejection {
    /// Токен неизвестен, истёк или относится к завершённой сессии.
    InvalidOrExpired,
    /// Сессия была явно отозвана на сервере.
    SessionRevoked,
    /// Уже потреблённый токен предъявлен повторно; сессия отозвана как скомпрометированная.
    Reused,
}

impl From<anyhow::Error> for AuthError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}
