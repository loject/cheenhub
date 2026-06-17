//! Доставка аутентификационных писем.

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Содержимое письма сброса пароля.
#[derive(Debug, Clone)]
pub(crate) struct PasswordResetEmail {
    /// Адрес email получателя.
    pub(crate) to: String,
    /// URL сброса, который откроет пользователь.
    pub(crate) reset_url: String,
}

/// Содержимое письма-уведомления о смене пароля.
#[derive(Debug, Clone)]
pub(crate) struct PasswordChangedEmail {
    /// Адрес email получателя.
    pub(crate) to: String,
}

/// Ошибка, возвращаемая доставкой аутентификационных писем.
#[derive(Debug)]
pub(crate) enum EmailError {
    /// Для доставки писем не хватает обязательных переменных окружения.
    Misconfigured {
        /// Имена отсутствующих переменных окружения.
        missing: Vec<&'static str>,
    },
    /// Доставка писем неожиданно завершилась ошибкой.
    Internal(anyhow::Error),
}

/// Отправитель аутентификационных писем.
#[async_trait]
pub(crate) trait AuthMailer: Send + Sync {
    /// Отправляет письмо сброса пароля.
    async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError>;

    /// Отправляет письмо-уведомление о смене пароля.
    async fn send_password_changed(&self, email: PasswordChangedEmail) -> Result<(), EmailError>;
}

/// Отправитель аутентификационных писем на базе SMTP.
pub(crate) struct SmtpAuthMailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: Option<String>,
    missing: Vec<&'static str>,
}

impl SmtpAuthMailer {
    /// Создает SMTP-отправитель писем аутентификации из конфигурации окружения.
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

    async fn send_password_changed(&self, email: PasswordChangedEmail) -> Result<(), EmailError> {
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
            .subject("CheenHub password changed")
            .body(password_changed_body())
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

fn password_changed_body() -> String {
    "Привет!\n\nПароль от аккаунта CheenHub был изменен. Если это был не ты, сразу запусти сброс пароля и проверь активные сеансы.\n".to_owned()
}

/// In-memory-отправитель писем для тестов.
#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::{AuthMailer, EmailError, PasswordChangedEmail, PasswordResetEmail};

    /// Тестовый отправитель писем аутентификации, который записывает отправленные письма сброса.
    #[derive(Default)]
    pub(crate) struct TestAuthMailer {
        sent: Mutex<Vec<PasswordResetEmail>>,
        password_changed: Mutex<Vec<PasswordChangedEmail>>,
    }

    impl TestAuthMailer {
        /// Возвращает отправленные письма сброса.
        pub(crate) fn sent(&self) -> Vec<PasswordResetEmail> {
            self.sent.lock().expect("test mailer lock").clone()
        }

        /// Возвращает уведомления о смене пароля.
        pub(crate) fn password_changed(&self) -> Vec<PasswordChangedEmail> {
            self.password_changed
                .lock()
                .expect("test mailer lock")
                .clone()
        }
    }

    #[async_trait]
    impl AuthMailer for TestAuthMailer {
        async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError> {
            self.sent.lock().expect("test mailer lock").push(email);
            Ok(())
        }

        async fn send_password_changed(
            &self,
            email: PasswordChangedEmail,
        ) -> Result<(), EmailError> {
            self.password_changed
                .lock()
                .expect("test mailer lock")
                .push(email);
            Ok(())
        }
    }
}
