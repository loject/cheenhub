//! Тесты независимых юридических подтверждений регистрации.

use cheenhub_contracts::rest::RegisterRequest;

use super::state;
use crate::features::auth::application::register;
use crate::features::auth::error::AuthError;

#[tokio::test]
async fn registration_requires_terms_acceptance() {
    let error = register(
        &state(),
        RegisterRequest {
            nickname: "without_terms".to_owned(),
            email: "without-terms@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: false,
            accepts_personal_data: true,
        },
    )
    .await
    .expect_err("registration without terms acceptance must fail");

    assert!(matches!(error, AuthError::BadRequest(message) if message.contains("соглашение")));
}

#[tokio::test]
async fn registration_requires_separate_personal_data_consent() {
    let error = register(
        &state(),
        RegisterRequest {
            nickname: "without_consent".to_owned(),
            email: "without-consent@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: false,
        },
    )
    .await
    .expect_err("registration without personal data consent must fail");

    assert!(matches!(error, AuthError::BadRequest(message) if message.contains("отдельно")));
}
