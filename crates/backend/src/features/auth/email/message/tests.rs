//! Тесты формирования MIME-сообщений аутентификации.

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
