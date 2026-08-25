//! REST-контракты глобальных настроек хоста CheenHub.

use serde::{Deserialize, Serialize};

/// Транспорт исходящих системных писем.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailTransport {
    /// Отправка через SMTP.
    Smtp,
    /// Отправка через Gmail API по HTTPS.
    GmailApi,
}

/// Результат проверки прав владельца хоста.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostAccessResponse {
    /// Имеет ли текущий пользователь права владельца этого хоста CheenHub.
    pub is_host_owner: bool,
}

/// Настройки отправки почты без сохранённых секретов.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HostEmailSettingsResponse {
    /// Активный транспорт отправки.
    pub transport: EmailTransport,
    /// Максимальное время одной операции отправки в секундах.
    pub email_send_timeout_seconds: u64,
    /// Адрес SMTP-сервера.
    pub smtp_host: Option<String>,
    /// Порт SMTP-сервера.
    pub smtp_port: u16,
    /// Имя пользователя SMTP.
    pub smtp_username: Option<String>,
    /// Настроен ли пароль SMTP.
    pub smtp_password_configured: bool,
    /// Адрес отправителя SMTP.
    pub smtp_from_email: Option<String>,
    /// ID OAuth-клиента Gmail.
    pub gmail_client_id: Option<String>,
    /// Взят ли ID OAuth-клиента из `GOOGLE_OAUTH_CLIENT_ID` вместо БД.
    pub gmail_client_id_from_environment: bool,
    /// Настроен ли секрет OAuth-клиента Gmail.
    pub gmail_client_secret_configured: bool,
    /// Взят ли секрет OAuth-клиента из `GOOGLE_OAUTH_CLIENT_SECRET` вместо БД.
    pub gmail_client_secret_from_environment: bool,
    /// Подключён ли Gmail и сохранён ли refresh token.
    pub gmail_connected: bool,
    /// Подтверждённый адрес подключённого Gmail-аккаунта.
    pub gmail_from_email: Option<String>,
    /// Точный callback URI, который нужно зарегистрировать в Google Cloud.
    pub gmail_oauth_redirect_uri: String,
}

/// Изменяемые настройки отправки почты владельцем хоста.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct UpdateHostEmailSettingsRequest {
    /// Новый активный транспорт, если его требуется изменить.
    pub transport: Option<EmailTransport>,
    /// Новый таймаут отправки от 1 до 300 секунд.
    pub email_send_timeout_seconds: Option<u64>,
    /// Новый адрес SMTP-сервера; пустая строка очищает поле.
    pub smtp_host: Option<String>,
    /// Новый порт SMTP-сервера.
    pub smtp_port: Option<u16>,
    /// Новое имя пользователя SMTP; пустая строка очищает поле.
    pub smtp_username: Option<String>,
    /// Новый пароль SMTP; пустая строка сохраняет прежний пароль.
    pub smtp_password: Option<String>,
    /// Явно удалить сохранённый пароль SMTP.
    pub clear_smtp_password: Option<bool>,
    /// Новый адрес отправителя SMTP; пустая строка очищает поле.
    pub smtp_from_email: Option<String>,
    /// Новый ID OAuth-клиента Gmail; пустая строка очищает поле.
    pub gmail_client_id: Option<String>,
    /// Новый секрет OAuth-клиента Gmail; пустая строка сохраняет прежний секрет.
    pub gmail_client_secret: Option<String>,
    /// Явно удалить сохранённый секрет OAuth-клиента Gmail.
    pub clear_gmail_client_secret: Option<bool>,
}

/// Результат запуска подключения Gmail.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GmailConnectionStartResponse {
    /// URL Google OAuth, на который следует перейти браузеру.
    pub authorization_url: String,
}

#[cfg(test)]
mod tests {
    use super::{EmailTransport, HostEmailSettingsResponse};

    #[test]
    fn email_settings_response_contains_only_secret_presence_flags() {
        let response = HostEmailSettingsResponse {
            transport: EmailTransport::GmailApi,
            email_send_timeout_seconds: 10,
            smtp_host: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_password_configured: true,
            smtp_from_email: None,
            gmail_client_id: Some("client-id".to_owned()),
            gmail_client_id_from_environment: false,
            gmail_client_secret_configured: true,
            gmail_client_secret_from_environment: false,
            gmail_connected: true,
            gmail_from_email: Some("sender@example.com".to_owned()),
            gmail_oauth_redirect_uri: "https://example.com/callback".to_owned(),
        };
        let json = serde_json::to_value(response).expect("response serializes");

        assert!(json.get("smtp_password").is_none());
        assert!(json.get("gmail_client_secret").is_none());
        assert!(json.get("gmail_refresh_token").is_none());
        assert_eq!(json["smtp_password_configured"], true);
        assert_eq!(json["gmail_client_secret_configured"], true);
        assert_eq!(json["gmail_connected"], true);
    }
}
