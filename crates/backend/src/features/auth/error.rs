//! Типы ошибок аутентификации.

/// Ошибка, возвращаемая потоками приложения аутентификации.
#[derive(Debug)]
pub(crate) enum AuthError {
    /// Данные запроса невалидны.
    BadRequest(String),
    /// Учетные данные или токены невалидны.
    Unauthorized(String),
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
            Self::Internal(_) => None,
        }
    }
}

impl From<anyhow::Error> for AuthError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}
