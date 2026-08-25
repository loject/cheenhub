//! Доменные данные глобальных настроек хоста.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Фиксированный идентификатор единственной строки настроек почты.
pub(crate) const EMAIL_SETTINGS_ID: Uuid = Uuid::nil();

/// Транспорт исходящих писем.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmailTransport {
    /// SMTP.
    Smtp,
    /// Gmail API по HTTPS.
    GmailApi,
}

impl EmailTransport {
    /// Возвращает строковое значение для БД и логов.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Smtp => "smtp",
            Self::GmailApi => "gmail_api",
        }
    }

    /// Разбирает значение из БД.
    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "smtp" => Ok(Self::Smtp),
            "gmail_api" => Ok(Self::GmailApi),
            _ => anyhow::bail!("unknown host email transport: {value}"),
        }
    }
}

/// Полные настройки отправки, включая секреты в открытом виде.
///
/// Временное хранение секретов без шифрования явно выбрано владельцем проекта.
#[derive(Clone, Debug)]
pub(crate) struct HostEmailSettings {
    pub(crate) transport: EmailTransport,
    pub(crate) email_send_timeout_seconds: u64,
    pub(crate) smtp_host: Option<String>,
    pub(crate) smtp_port: u16,
    pub(crate) smtp_username: Option<String>,
    pub(crate) smtp_password: Option<String>,
    pub(crate) smtp_from_email: Option<String>,
    pub(crate) gmail_client_id: Option<String>,
    pub(crate) gmail_client_secret: Option<String>,
    pub(crate) gmail_refresh_token: Option<String>,
    pub(crate) gmail_from_email: Option<String>,
}

impl Default for HostEmailSettings {
    fn default() -> Self {
        Self {
            transport: EmailTransport::Smtp,
            email_send_timeout_seconds: 10,
            smtp_host: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
            smtp_from_email: None,
            gmail_client_id: None,
            gmail_client_secret: None,
            gmail_refresh_token: None,
            gmail_from_email: None,
        }
    }
}

impl HostEmailSettings {
    /// Подставляет OAuth-реквизиты из окружения только вместо отсутствующих значений БД.
    pub(crate) fn with_gmail_oauth_fallback(
        mut self,
        client_id: Option<String>,
        client_secret: Option<String>,
    ) -> Self {
        self.gmail_client_id = self.gmail_client_id.or(client_id);
        self.gmail_client_secret = self.gmail_client_secret.or(client_secret);
        self
    }
}

/// Одноразовое состояние подключения Gmail.
pub(crate) struct GmailOAuthState {
    pub(crate) id: Uuid,
    pub(crate) state_hash: String,
    pub(crate) user_id: Uuid,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::HostEmailSettings;

    #[test]
    fn database_gmail_credentials_override_environment_fallback() {
        let settings = HostEmailSettings {
            gmail_client_id: Some("database-id".to_owned()),
            gmail_client_secret: None,
            ..HostEmailSettings::default()
        }
        .with_gmail_oauth_fallback(
            Some("environment-id".to_owned()),
            Some("environment-secret".to_owned()),
        );

        assert_eq!(settings.gmail_client_id.as_deref(), Some("database-id"));
        assert_eq!(
            settings.gmail_client_secret.as_deref(),
            Some("environment-secret")
        );
    }
}
