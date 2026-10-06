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

/// Тестовый double отправителя писем аутентификации.
#[cfg(test)]
pub(crate) mod tests;
