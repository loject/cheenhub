//! Authentication email delivery.

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Password reset email content.
#[derive(Debug, Clone)]
pub(crate) struct PasswordResetEmail {
    /// Destination email address.
    pub(crate) to: String,
    /// Reset URL opened by the user.
    pub(crate) reset_url: String,
}

/// Error returned by authentication email delivery.
#[derive(Debug)]
pub(crate) enum EmailError {
    /// Email delivery is missing required environment variables.
    Misconfigured {
        /// Missing environment variable names.
        missing: Vec<&'static str>,
    },
    /// Email delivery failed unexpectedly.
    Internal(anyhow::Error),
}

/// Authentication email sender.
#[async_trait]
pub(crate) trait AuthMailer: Send + Sync {
    /// Sends a password reset email.
    async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError>;
}

/// SMTP-backed authentication email sender.
pub(crate) struct SmtpAuthMailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: Option<String>,
    missing: Vec<&'static str>,
}

impl SmtpAuthMailer {
    /// Builds an SMTP auth mailer from environment configuration.
    pub(crate) fn new(
        host: Option<String>,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        from: Option<String>,
    ) -> anyhow::Result<Self> {
        let missing = missing_smtp_config(&host, &username, &password, &from);
        if !missing.is_empty() {
            return Ok(Self {
                transport: None,
                from,
                missing,
            });
        }

        let host = host.expect("host was checked");
        let username = username.expect("username was checked");
        let password = password.expect("password was checked");
        let transport = smtp_transport_builder(&host, port)?
            .port(port)
            .credentials(Credentials::new(username, password))
            .build();

        Ok(Self {
            transport: Some(transport),
            from,
            missing,
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
        let Some(transport) = &self.transport else {
            return Err(EmailError::Misconfigured {
                missing: self.missing.clone(),
            });
        };
        let from = self
            .from
            .as_ref()
            .ok_or_else(|| EmailError::Misconfigured {
                missing: vec!["SMTP_FROM_EMAIL"],
            })?;

        let message = Message::builder()
            .from(parse_mailbox(from)?)
            .to(parse_mailbox(&email.to)?)
            .subject("CheenHub password reset")
            .body(password_reset_body(&email.reset_url))
            .map_err(|error| EmailError::Internal(error.into()))?;

        transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|error| EmailError::Internal(error.into()))
    }
}

fn missing_smtp_config(
    host: &Option<String>,
    username: &Option<String>,
    password: &Option<String>,
    from: &Option<String>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if host.as_deref().is_none_or(str::is_empty) {
        missing.push("SMTP_HOST");
    }
    if username.as_deref().is_none_or(str::is_empty) {
        missing.push("SMTP_USERNAME");
    }
    if password.as_deref().is_none_or(str::is_empty) {
        missing.push("SMTP_PASSWORD");
    }
    if from.as_deref().is_none_or(str::is_empty) {
        missing.push("SMTP_FROM_EMAIL");
    }
    missing
}

fn parse_mailbox(value: &str) -> Result<Mailbox, EmailError> {
    value
        .parse()
        .map_err(|error| EmailError::Internal(anyhow::anyhow!("invalid email address: {error}")))
}

fn password_reset_body(reset_url: &str) -> String {
    format!(
        "Привет!\n\nЧтобы сбросить пароль CheenHub, открой ссылку:\n{reset_url}\n\nЕсли ты не запрашивал сброс пароля, просто проигнорируй это письмо.\n"
    )
}

/// In-memory email sender for tests.
#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::{AuthMailer, EmailError, PasswordResetEmail};

    /// Test auth mailer that records sent reset emails.
    #[derive(Default)]
    pub(crate) struct TestAuthMailer {
        sent: Mutex<Vec<PasswordResetEmail>>,
    }

    impl TestAuthMailer {
        /// Returns sent reset emails.
        pub(crate) fn sent(&self) -> Vec<PasswordResetEmail> {
            self.sent.lock().expect("test mailer lock").clone()
        }
    }

    #[async_trait]
    impl AuthMailer for TestAuthMailer {
        async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError> {
            self.sent.lock().expect("test mailer lock").push(email);
            Ok(())
        }
    }
}
