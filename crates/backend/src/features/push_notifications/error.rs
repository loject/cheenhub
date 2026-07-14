//! Ошибки регистрации системных push-уведомлений.

/// Ошибка REST-сценария push-уведомлений.
#[derive(Debug)]
pub(crate) enum PushError {
    /// Запрос содержит некорректные данные.
    BadRequest(String),
    /// Access token отсутствует или недействителен.
    Unauthorized(String),
    /// Установка не найдена в текущей auth-сессии.
    NotFound(String),
    /// Хранилище push-уведомлений недоступно в текущем режиме backend.
    Unavailable(String),
    /// Непредвиденный сбой инфраструктуры.
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for PushError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}
