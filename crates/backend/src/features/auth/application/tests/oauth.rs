//! OAuth authentication application tests.

use cheenhub_contracts::rest::{
    LoginRequest, OAuthCompleteRequest, OAuthCompleteResponse, OAuthRegistrationRequest,
};
use chrono::{Duration, Utc};

use super::{google_only_user, registered_user, state};
use crate::features::auth::application::{
    complete_google_oauth, login, register_with_google_oauth, unlink_google,
};
use crate::features::auth::security::refresh_token;

#[tokio::test]
async fn google_registration_intent_creates_passwordless_account() {
    let state = state();
    let now = Utc::now();
    let handoff_code = refresh_token::generate();
    let intent = state
        .auth_store
        .insert_oauth_registration_intent(
            "google".to_owned(),
            "google-subject-1".to_owned(),
            "new-google@example.com".to_owned(),
            Some("New Google".to_owned()),
            now,
            now + Duration::minutes(15),
        )
        .await
        .expect("intent should insert");
    state
        .auth_store
        .insert_oauth_handoff(
            refresh_token::hash(&handoff_code),
            "registration_required".to_owned(),
            None,
            Some(intent.id),
            now,
            now + Duration::minutes(5),
        )
        .await
        .expect("handoff should insert");

    let auth = register_with_google_oauth(
        &state,
        OAuthRegistrationRequest {
            registration_token: handoff_code,
            nickname: "google_user".to_owned(),
            accepts_policies: true,
        },
        None,
    )
    .await
    .expect("google registration should succeed");

    assert_eq!(auth.user.email, "new-google@example.com");
    let password_login = login(
        &state,
        LoginRequest {
            email: "new-google@example.com".to_owned(),
            password: "password123".to_owned(),
        },
    )
    .await;
    assert!(password_login.is_err());
}

#[tokio::test]
async fn linked_google_handoff_logs_user_in() {
    let state = state();
    let auth = registered_user(&state, "linked_google", "linked-google@example.com").await;
    let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should parse");
    let now = Utc::now();
    let handoff_code = refresh_token::generate();
    state
        .auth_store
        .insert_oauth_account(
            &user_id,
            "google".to_owned(),
            "google-subject-2".to_owned(),
            "linked-google@example.com".to_owned(),
            Some("Linked Google".to_owned()),
            now,
        )
        .await
        .expect("oauth account should insert");
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

    let complete = complete_google_oauth(&state, OAuthCompleteRequest { handoff_code }, None)
        .await
        .expect("handoff should complete");

    match complete {
        OAuthCompleteResponse::Authenticated { auth } => {
            assert_eq!(auth.user.email, "linked-google@example.com");
        }
        _ => panic!("expected authenticated handoff"),
    }
}

#[tokio::test]
async fn expired_google_handoff_is_rejected() {
    let state = state();
    let now = Utc::now();
    let handoff_code = refresh_token::generate();
    state
        .auth_store
        .insert_oauth_handoff(
            refresh_token::hash(&handoff_code),
            "authenticated".to_owned(),
            Some(uuid::Uuid::new_v4()),
            None,
            now - Duration::minutes(10),
            now - Duration::minutes(5),
        )
        .await
        .expect("handoff should insert");

    let result = complete_google_oauth(&state, OAuthCompleteRequest { handoff_code }, None).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn google_only_account_cannot_unlink_google() {
    let state = state();
    let auth = google_only_user(&state).await;

    let result = unlink_google(&state, &auth.access_token).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn password_account_can_unlink_google() {
    let state = state();
    let auth = registered_user(&state, "unlink_google", "unlink-google@example.com").await;
    let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should parse");
    state
        .auth_store
        .insert_oauth_account(
            &user_id,
            "google".to_owned(),
            "google-subject-3".to_owned(),
            "unlink-google@example.com".to_owned(),
            None,
            Utc::now(),
        )
        .await
        .expect("oauth account should insert");

    let linked = unlink_google(&state, &auth.access_token)
        .await
        .expect("unlink should succeed");

    assert!(linked.accounts.is_empty());
}
