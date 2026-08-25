//! Динамическая доставка писем согласно настройкам хоста.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::features::auth::email::{
    AuthMailer, EmailError, GmailApiAuthMailer, PasswordChangedEmail, PasswordResetEmail,
    SmtpAuthMailer,
};

use super::{domain::EmailTransport, infrastructure::HostSettingsStore};

/// Отправитель, выбирающий актуальный транспорт из БД перед каждым письмом.
pub(crate) struct DynamicAuthMailer {
    settings_store: Arc<dyn HostSettingsStore>,
    gmail_client_id_fallback: Option<String>,
    gmail_client_secret_fallback: Option<String>,
}

impl DynamicAuthMailer {
    /// Создаёт динамический отправитель поверх хранилища настроек хоста.
    pub(crate) fn new(
        settings_store: Arc<dyn HostSettingsStore>,
        gmail_client_id_fallback: Option<String>,
        gmail_client_secret_fallback: Option<String>,
    ) -> Self {
        Self {
            settings_store,
            gmail_client_id_fallback,
            gmail_client_secret_fallback,
        }
    }

    async fn current(&self) -> Result<Box<dyn AuthMailer>, EmailError> {
        let settings = self
            .settings_store
            .load_email_settings()
            .await
            .map_err(EmailError::Internal)?
            .with_gmail_oauth_fallback(
                self.gmail_client_id_fallback.clone(),
                self.gmail_client_secret_fallback.clone(),
            );
        let timeout = Duration::from_secs(settings.email_send_timeout_seconds);
        match settings.transport {
            EmailTransport::Smtp => Ok(Box::new(
                SmtpAuthMailer::new(
                    settings.smtp_host,
                    settings.smtp_port,
                    settings.smtp_username,
                    settings.smtp_password,
                    settings.smtp_from_email,
                    timeout,
                )
                .map_err(EmailError::Internal)?,
            )),
            EmailTransport::GmailApi => Ok(Box::new(
                GmailApiAuthMailer::new(
                    settings.gmail_client_id,
                    settings.gmail_client_secret,
                    settings.gmail_refresh_token,
                    settings.gmail_from_email,
                    timeout,
                )
                .map_err(EmailError::Internal)?,
            )),
        }
    }
}

#[async_trait]
impl AuthMailer for DynamicAuthMailer {
    async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError> {
        self.current().await?.send_password_reset(email).await
    }

    async fn send_password_changed(&self, email: PasswordChangedEmail) -> Result<(), EmailError> {
        self.current().await?.send_password_changed(email).await
    }
}
