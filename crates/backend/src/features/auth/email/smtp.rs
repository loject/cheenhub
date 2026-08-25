//! SMTP-транспорт аутентификационных писем.

use std::time::Duration;

use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use super::message::{password_changed_message, password_reset_message};
use super::{AuthMailer, EmailError, PasswordChangedEmail, PasswordResetEmail};

/// Отправитель аутентификационных писем на базе SMTP.
pub(crate) struct SmtpAuthMailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: Option<String>,
    missing: Vec<&'static str>,
    timeout: Duration,
}

impl SmtpAuthMailer {
    /// Создает SMTP-отправитель из выбранных настроек хоста.
    pub(crate) fn new(
        host: Option<String>,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        from: Option<String>,
        timeout: Duration,
    ) -> anyhow::Result<Self> {
        let missing = missing_smtp_config(&host, &username, &password, &from);
        if !missing.is_empty() {
            tracing::warn!(
                email_transport = "smtp",
                missing_fields = ?missing,
                "auth email transport is not fully configured"
            );
            return Ok(Self {
                transport: None,
                from,
                missing,
                timeout,
            });
        }

        let host = host.expect("host was checked");
        let username = username.expect("username was checked");
        let password = password.expect("password was checked");
        let transport = smtp_transport_builder(&host, port)?
            .port(port)
            .credentials(Credentials::new(username, password))
            .timeout(Some(timeout))
            .build();

        Ok(Self {
            transport: Some(transport),
            from,
            missing,
            timeout,
        })
    }

    async fn send(&self, message: lettre::Message, kind: &'static str) -> Result<(), EmailError> {
        let transport = self.configured_transport()?;
        tokio::time::timeout(self.timeout, transport.send(message))
            .await
            .map_err(|_| {
                EmailError::Internal(anyhow::anyhow!(
                    "SMTP email send timed out after {} seconds",
                    self.timeout.as_secs()
                ))
            })?
            .map_err(|error| EmailError::Internal(error.into()))?;
        tracing::info!(
            email_transport = "smtp",
            email_kind = kind,
            "auth email sent"
        );
        Ok(())
    }

    fn configured_transport(&self) -> Result<&AsyncSmtpTransport<Tokio1Executor>, EmailError> {
        self.transport
            .as_ref()
            .ok_or_else(|| EmailError::Misconfigured {
                missing: self.missing.clone(),
            })
    }

    fn from(&self) -> Result<&str, EmailError> {
        self.from
            .as_deref()
            .ok_or_else(|| EmailError::Misconfigured {
                missing: vec!["smtp_from_email"],
            })
    }
}

fn smtp_transport_builder(
    host: &str,
    port: u16,
) -> Result<lettre::transport::smtp::AsyncSmtpTransportBuilder, lettre::transport::smtp::Error> {
    if port == 465 {
        return AsyncSmtpTransport::<Tokio1Executor>::relay(host);
    }

    AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
}

#[async_trait]
impl AuthMailer for SmtpAuthMailer {
    async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError> {
        let message = password_reset_message(self.from()?, &email.to, &email.reset_url)?;
        self.send(message, "password_reset").await
    }

    async fn send_password_changed(&self, email: PasswordChangedEmail) -> Result<(), EmailError> {
        let message = password_changed_message(self.from()?, &email.to)?;
        self.send(message, "password_changed").await
    }
}

fn missing_smtp_config(
    host: &Option<String>,
    username: &Option<String>,
    password: &Option<String>,
    from: &Option<String>,
) -> Vec<&'static str> {
    let values = [
        ("smtp_host", host),
        ("smtp_username", username),
        ("smtp_password", password),
        ("smtp_from_email", from),
    ];
    values
        .into_iter()
        .filter_map(|(key, value)| value.as_deref().is_none_or(str::is_empty).then_some(key))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::SmtpAuthMailer;

    #[test]
    fn reports_missing_smtp_settings() {
        let mailer = SmtpAuthMailer::new(None, 587, None, None, None, Duration::from_secs(10))
            .expect("неполная конфигурация должна сохраняться как отключенный mailer");
        assert_eq!(
            mailer.missing,
            [
                "smtp_host",
                "smtp_username",
                "smtp_password",
                "smtp_from_email"
            ]
        );
    }
}
