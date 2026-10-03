//! Тестовый double отправителя писем аутентификации.

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
    async fn send_account_deletion(&self, email: AccountDeletionEmail) -> Result<(), EmailError> {
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

    async fn send_password_changed(&self, email: PasswordChangedEmail) -> Result<(), EmailError> {
        self.password_changed
            .lock()
            .expect("test mailer lock")
            .push(email);
        Ok(())
    }
}
