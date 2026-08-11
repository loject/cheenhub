//! Тесты атомарного потребления одноразовых auth-токенов.

use cheenhub_contracts::rest::{
    OAuthCompleteRequest, PasswordResetConfirmRequest, PasswordResetRequest,
};
use chrono::{Duration, Utc};

use super::{registered_user, reset_token_from_mailer, state, state_with_mailer};
use crate::features::auth::application::{
    complete_google_oauth, confirm_password_reset, request_password_reset,
};
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;

#[tokio::test]
async fn newer_password_reset_request_invalidates_previous_link() {
    let (state, mailer) = state_with_mailer();
    registered_user(&state, "exclusive_reset", "exclusive-reset@example.com").await;
    let request = PasswordResetRequest {
        email: "exclusive-reset@example.com".to_owned(),
    };
    request_password_reset(&state, request.clone())
        .await
        .expect("first password reset request should succeed");
    let first_token = reset_token_from_mailer(&mailer);
    request_password_reset(&state, request)
        .await
        .expect("second password reset request should succeed");
    let second_token = reset_token_from_mailer(&mailer);

    let first_result = confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token: first_token,
            new_password: "first-new-password123".to_owned(),
        },
    )
    .await;
    assert!(matches!(first_result, Err(AuthError::Unauthorized(_))));

    confirm_password_reset(
        &state,
        PasswordResetConfirmRequest {
            token: second_token,
            new_password: "second-new-password123".to_owned(),
        },
    )
    .await
    .expect("latest reset link should remain valid");
}

#[tokio::test]
async fn concurrent_password_reset_confirmation_has_one_winner() {
    let (state, mailer) = state_with_mailer();
    registered_user(&state, "atomic_reset", "atomic-reset@example.com").await;
    request_password_reset(
        &state,
        PasswordResetRequest {
            email: "atomic-reset@example.com".to_owned(),
        },
    )
    .await
    .expect("password reset request should succeed");
    let token = reset_token_from_mailer(&mailer);

    let (first, second) = tokio::join!(
        confirm_password_reset(
            &state,
            PasswordResetConfirmRequest {
                token: token.clone(),
                new_password: "first-race-password123".to_owned(),
            },
        ),
        confirm_password_reset(
            &state,
            PasswordResetConfirmRequest {
                token,
                new_password: "second-race-password123".to_owned(),
            },
        ),
    );

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(
        [first, second]
            .into_iter()
            .filter_map(Result::err)
            .all(|error| matches!(error, AuthError::Unauthorized(_)))
    );
}

#[tokio::test]
async fn concurrent_google_handoff_completion_has_one_winner() {
    let state = state();
    let auth = registered_user(&state, "atomic_google", "atomic-google@example.com").await;
    let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should parse");
    let now = Utc::now();
    let handoff_code = refresh_token::generate();
    state
        .auth_store
        .insert_oauth_handoff(
            refresh_token::hash(&handoff_code),
            "authenticated".to_owned(),
            Some(user_id),
            None,
            now,
            now + Duration::minutes(5),
        )
        .await
        .expect("handoff should insert");

    let (first, second) = tokio::join!(
        complete_google_oauth(
            &state,
            OAuthCompleteRequest {
                handoff_code: handoff_code.clone(),
            },
            None,
        ),
        complete_google_oauth(&state, OAuthCompleteRequest { handoff_code }, None,),
    );

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
}
