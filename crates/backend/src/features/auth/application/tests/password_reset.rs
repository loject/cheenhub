//! Сценарии сброса пароля по email.

use cheenhub_contracts::rest::{LoginRequest, PasswordResetConfirmRequest, PasswordResetRequest};
use chrono::{Duration, Utc};

use super::support::{
    google_only_user, register_test_session, registered_user, reset_token_from_mailer, state,
    state_with_mailer,
};
use crate::features::auth::application::{
    confirm_password_reset, login, me, request_password_reset,
};
use crate::features::auth::security::refresh_token;

#[tokio::test]
async fn password_reset_request_sends_email_for_existing_user() {
    let (state, mailer) = state_with_mailer();
    registered_user(&state, "reset_user", "reset-user@example.com").await;

    request_password_reset(
        &state,
        PasswordResetRequest {
            email: "reset-user@example.com".to_owned(),
        },
    )
    .await
    .expect("password reset request should succeed");

    let sent = mailer.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "reset-user@example.com");
    assert!(sent[0].reset_url.contains("/reset-password?token="));
}

#[tokio::test]
async fn password_reset_request_for_unknown_email_sends_nothing() {
    let (state, mailer) = state_with_mailer();

    request_password_reset(
        &state,
        PasswordResetRequest {
            email: "missing@example.com".to_owned(),
        },
    )
    .await
    .expect("password reset request should be neutral");

    assert!(mailer.sent().is_empty());
}

#[tokio::test]
async fn password_reset_confirm_changes_password() {
    let (state, mailer) = state_with_mailer();
    registered_user(&state, "change_password", "change-password@example.com").await;
    request_password_reset(
        &state,
        PasswordResetRequest {
            email: "change-password@example.com".to_owned(),
        },
    )
    .await
    .expect("password reset request should succeed");
    let token = reset_token_from_mailer(&mailer);

    confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token,
            new_password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("password reset confirm should succeed");

    let old_login = login(
        &state,
        LoginRequest {
            email: "change-password@example.com".to_owned(),
            password: "password123".to_owned(),
        },
    )
    .await;
    assert!(old_login.is_err());

    let new_login = login(
        &state,
        LoginRequest {
            email: "change-password@example.com".to_owned(),
            password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("new password should work");
    assert_eq!(new_login.user.email, "change-password@example.com");
}

#[tokio::test]
async fn consumed_password_reset_token_is_rejected() {
    let (state, mailer) = state_with_mailer();
    registered_user(&state, "used_reset", "used-reset@example.com").await;
    request_password_reset(
        &state,
        PasswordResetRequest {
            email: "used-reset@example.com".to_owned(),
        },
    )
    .await
    .expect("password reset request should succeed");
    let token = reset_token_from_mailer(&mailer);

    confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token: token.clone(),
            new_password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("first confirm should succeed");
    let second = confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token,
            new_password: "another-password123".to_owned(),
        },
    )
    .await;

    assert!(second.is_err());
}

#[tokio::test]
async fn expired_password_reset_token_is_rejected() {
    let state = state();
    let auth = registered_user(&state, "expired_reset", "expired-reset@example.com").await;
    let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should parse");
    let reset_token = refresh_token::generate();
    let now = Utc::now();
    state
        .auth_store
        .insert_password_reset_token(
            &user_id,
            refresh_token::hash(&reset_token),
            now - Duration::minutes(10),
            now - Duration::minutes(5),
        )
        .await
        .expect("reset token should insert");

    let result = confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token: reset_token,
            new_password: "new-password123".to_owned(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn password_reset_revokes_existing_sessions() {
    let (state, mailer) = state_with_mailer();
    let auth = registered_user(&state, "revoke_reset", "revoke-reset@example.com").await;
    let realtime_disconnect = register_test_session(&state, &auth).await;
    request_password_reset(
        &state,
        PasswordResetRequest {
            email: "revoke-reset@example.com".to_owned(),
        },
    )
    .await
    .expect("password reset request should succeed");
    let token = reset_token_from_mailer(&mailer);

    confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token,
            new_password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("password reset confirm should succeed");
    assert_eq!(
        *realtime_disconnect.borrow(),
        Some(crate::realtime::hub::DisconnectReason::AuthSessionRevoked)
    );

    let current_user = me(&state, &auth.access_token).await;
    assert!(current_user.is_err());
}

#[tokio::test]
async fn oauth_only_account_can_set_first_password_through_reset() {
    let (state, mailer) = state_with_mailer();
    google_only_user(&state).await;
    request_password_reset(
        &state,
        PasswordResetRequest {
            email: "google-only@example.com".to_owned(),
        },
    )
    .await
    .expect("password reset request should succeed");
    let token = reset_token_from_mailer(&mailer);

    confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token,
            new_password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("password reset confirm should succeed");

    let auth = login(
        &state,
        LoginRequest {
            email: "google-only@example.com".to_owned(),
            password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("new password should work");
    assert_eq!(auth.user.email, "google-only@example.com");
}
