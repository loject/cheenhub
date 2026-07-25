//! Платформенный вход через Google.

use std::fmt;

mod api;
mod native;

use super::api::OAuthCompletion;

/// Ошибка запуска или обработки системного входа через Google.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct GoogleSignInError(String);

impl GoogleSignInError {
    /// Создаёт ошибку с безопасным для журнала описанием.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for GoogleSignInError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for GoogleSignInError {}

/// Сообщает, доступен ли системный вход через Google на текущей платформе.
pub(crate) const fn is_supported() -> bool {
    native::is_supported()
}

/// Открывает системный Android Credential Manager и возвращает Google ID token.
pub(crate) async fn request_google_id_token(
    server_client_id: String,
    nonce: String,
) -> Result<Option<String>, GoogleSignInError> {
    native::request_google_id_token(server_client_id, nonce).await
}

/// Выполняет полный нативный вход и возвращает серверный результат OAuth.
pub(crate) async fn authenticate() -> Result<Option<OAuthCompletion>, String> {
    let start = api::start().await?;
    match request_google_id_token(start.server_client_id, start.nonce)
        .await
        .map_err(|error| error.to_string())?
    {
        Some(id_token) => api::complete(start.challenge, id_token).await.map(Some),
        None => Ok(None),
    }
}
