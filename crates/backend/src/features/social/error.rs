//! Ошибки social-функций.

/// Ошибка потоков приложения друзей и личных сообщений.
#[derive(Debug)]
pub(crate) enum SocialError {
    /// Неверная форма запроса.
    BadRequest(String),
    /// Сессия недействительна или не передана.
    Unauthorized(String),
    /// Запрошенный ресурс не найден или недоступен пользователю.
    NotFound(String),
    /// Запрос конфликтует с текущим состоянием связи.
    Conflict(String),
    /// Непредвиденный сбой инфраструктуры.
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for SocialError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}
