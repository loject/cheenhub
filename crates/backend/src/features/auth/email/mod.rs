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

/// Содержимое уведомления об удалении аккаунта с возможностью восстановления.
#[derive(Debug, Clone)]
pub(crate) struct AccountDeletionEmail {
    /// Адрес email получателя.
    pub(crate) to: String,
    /// URL восстановления аккаунта.
    pub(crate) restore_url: String,
    /// Крайний срок восстановления с указанием часового пояса.
    pub(crate) restore_until: String,
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

    /// Отправляет уведомление об удалении аккаунта и сроке восстановления.
    async fn send_account_deletion(&self, email: AccountDeletionEmail) -> Result<(), EmailError>;

    /// Отправляет письмо-уведомление о смене пароля.
    async fn send_password_changed(&self, email: PasswordChangedEmail) -> Result<(), EmailError>;
}

/// In-memory-отправитель писем для тестов.
#[cfg(test)]
pub(crate) mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use async_trait::async_trait;

    use super::{
        AccountDeletionEmail, AuthMailer, EmailError, PasswordChangedEmail, PasswordResetEmail,
    };

    /// Тестовый отправитель писем аутентификации, который записывает отправленные письма сброса.
    #[derive(Default)]
    pub(crate) struct TestAuthMailer {
        sent: Mutex<Vec<PasswordResetEmail>>,
        password_changed: Mutex<Vec<PasswordChangedEmail>>,
        account_deletion: Mutex<Vec<AccountDeletionEmail>>,
        fail_account_deletion: AtomicBool,
    }

    impl TestAuthMailer {
        /// Возвращает отправленные письма сброса.
        pub(crate) fn sent(&self) -> Vec<PasswordResetEmail> {
            self.sent.lock().expect("test mailer lock").clone()
        }

        /// Включает отказ доставки уведомления об удалении аккаунта.
        pub(crate) fn fail_account_deletion(&self) {
            self.fail_account_deletion.store(true, Ordering::SeqCst);
        }

        /// Возвращает уведомления об удалении аккаунта.
        pub(crate) fn account_deletion(&self) -> Vec<AccountDeletionEmail> {
            self.account_deletion
                .lock()
                .expect("test mailer lock")
                .clone()
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
        async fn send_account_deletion(
            &self,
            email: AccountDeletionEmail,
        ) -> Result<(), EmailError> {
            if self.fail_account_deletion.load(Ordering::SeqCst) {
                return Err(EmailError::Internal(anyhow::anyhow!(
                    "test delivery failure"
                )));
            }
            self.account_deletion
                .lock()
                .expect("test mailer lock")
                .push(email);
            Ok(())
        }

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
