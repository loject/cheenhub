//! Доставка аутентификационных писем.

mod gmail_api;
mod message;
mod smtp;

use async_trait::async_trait;
pub(crate) use gmail_api::GmailApiAuthMailer;
pub(crate) use smtp::SmtpAuthMailer;

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
    /// Для доставки писем не хватает обязательных полей настройки.
    Misconfigured {
        /// Имена отсутствующих полей.
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
