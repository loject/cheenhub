//! Realtime client error types.

/// Realtime client error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RealtimeError {
    message: String,
    user_message: Option<String>,
}

impl RealtimeError {
    /// Creates a realtime client error.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            user_message: None,
        }
    }

    /// Создаёт отказ сервера с текстом, предназначенным для пользователя.
    pub(crate) fn rejected(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            user_message: Some(message.clone()),
            message,
        }
    }

    /// Возвращает пользовательский текст отказа; технические ошибки его не имеют.
    pub(crate) fn user_message(&self) -> Option<&str> {
        self.user_message.as_deref()
    }
}

impl std::fmt::Display for RealtimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RealtimeError {}
