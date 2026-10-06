//! Тесты SMTP-отправителя писем аутентификации.

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
