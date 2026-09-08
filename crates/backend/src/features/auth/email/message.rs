//! Формирование общих MIME-сообщений для почтовых транспортов.

use lettre::{Message, message::Mailbox};

use super::EmailError;

pub(super) fn password_reset_message(
    from: &str,
    to: &str,
    reset_url: &str,
) -> Result<Message, EmailError> {
    build_message(
        from,
        to,
        "CheenHub password reset",
        format!(
            "Привет!\n\nЧтобы сбросить пароль CheenHub, открой ссылку:\n{reset_url}\n\nЕсли ты не запрашивал сброс пароля, просто проигнорируй это письмо.\n"
        ),
    )
}

pub(super) fn password_changed_message(from: &str, to: &str) -> Result<Message, EmailError> {
    build_message(
        from,
        to,
        "CheenHub password changed",
        "Привет!\n\nПароль от аккаунта CheenHub был изменен. Если это был не ты, сразу запусти сброс пароля и проверь активные сеансы.\n".to_owned(),
    )
}

pub(super) fn account_deletion_message(
    from: &str,
    to: &str,
    restore_url: &str,
    restore_until: &str,
) -> Result<Message, EmailError> {
    build_message(
        from,
        to,
        "Удаление аккаунта CheenHub",
        account_deletion_body(restore_url, restore_until),
    )
}

fn account_deletion_body(restore_url: &str, restore_until: &str) -> String {
    format!(
        "Привет!\n\nУдаление твоего аккаунта CheenHub запрошено. У тебя есть 30 дней, чтобы отменить удаление и восстановить аккаунт.\n\nВосстановление доступно до {restore_until}. После этого срока восстановить аккаунт будет невозможно.\n\nЧтобы восстановить аккаунт, открой ссылку:\n{restore_url}\n\nЕсли ты не запрашивал удаление, восстанови аккаунт и смени пароль.\n"
    )
}

fn build_message(from: &str, to: &str, subject: &str, body: String) -> Result<Message, EmailError> {
    Message::builder()
        .from(parse_mailbox(from)?)
        .to(parse_mailbox(to)?)
        .subject(subject)
        .body(body)
        .map_err(|error| EmailError::Internal(error.into()))
}

fn parse_mailbox(value: &str) -> Result<Mailbox, EmailError> {
    value
        .parse()
        .map_err(|error| EmailError::Internal(anyhow::anyhow!("invalid email address: {error}")))
}

#[cfg(test)]
mod tests {
    use super::{account_deletion_body, account_deletion_message};

    #[test]
    fn deletion_email_contains_restore_link_and_deadline() {
        let restore_url = "https://cheenhub.test/restore-account?token=test-token";
        let restore_until = "2026-10-05 12:00 UTC";
        let message = account_deletion_message(
            "sender@example.com",
            "recipient@example.com",
            restore_url,
            restore_until,
        )
        .expect("письмо должно собираться");
        let formatted = String::from_utf8(message.formatted()).expect("MIME должен быть UTF-8");
        assert!(formatted.contains("From: sender@example.com"));
        assert!(formatted.contains("To: recipient@example.com"));
        let body = account_deletion_body(restore_url, restore_until);
        assert!(body.contains(restore_url));
        assert!(body.contains(restore_until));
        assert!(body.contains("30 дней"));
    }
}
