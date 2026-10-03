//! Тесты Gmail API-отправителя писем аутентификации.

use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

use super::{GmailApiAuthMailer, encode_raw_message};
use crate::features::auth::email::message::password_reset_message;

#[test]
fn encodes_rfc_2822_message_as_unpadded_base64url() {
    let message = password_reset_message(
        "sender@example.com",
        "recipient@example.com",
        "https://cheenhub.test/reset?token=test",
    )
    .expect("письмо должно собираться");
    let encoded = encode_raw_message(&message);
    assert!(!encoded.contains(['+', '/', '=']));

    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .expect("raw должен декодироваться как base64url");
    let decoded = String::from_utf8(decoded).expect("сообщение должно быть UTF-8");
    assert!(decoded.contains("Subject: CheenHub password reset"));
    assert!(decoded.contains("From: sender@example.com"));
    assert!(decoded.contains("To: recipient@example.com"));
}

#[test]
fn reports_missing_gmail_api_settings() {
    let mailer = GmailApiAuthMailer::new(None, None, None, None, Duration::from_secs(10))
        .expect("неполная конфигурация должна сохраняться как отключенный mailer");
    assert_eq!(
        mailer.missing,
        [
            "gmail_client_id",
            "gmail_client_secret",
            "gmail_refresh_token",
            "gmail_from_email"
        ]
    );
}
