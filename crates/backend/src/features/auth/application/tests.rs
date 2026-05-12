//! Authentication application tests.

use std::sync::Arc;

use cheenhub_contracts::rest::{
    LoginRequest, OAuthCompleteRequest, OAuthCompleteResponse, OAuthRegistrationRequest,
    RegisterRequest,
};
use chrono::{Duration, Utc};

use super::{complete_google_oauth, login, register, register_with_google_oauth, unlink_google};
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::{keys::AuthKeys, refresh_token};
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

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

    let complete = complete_google_oauth(&state, OAuthCompleteRequest { handoff_code })
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

    let result = complete_google_oauth(&state, OAuthCompleteRequest { handoff_code }).await;

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

async fn registered_user(
    state: &AppState,
    nickname: &str,
    email: &str,
) -> cheenhub_contracts::rest::AuthResponse {
    register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed")
}

async fn google_only_user(state: &AppState) -> cheenhub_contracts::rest::AuthResponse {
    let now = Utc::now();
    let handoff_code = refresh_token::generate();
    let intent = state
        .auth_store
        .insert_oauth_registration_intent(
            "google".to_owned(),
            "google-subject-only".to_owned(),
            "google-only@example.com".to_owned(),
            None,
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

    register_with_google_oauth(
        state,
        OAuthRegistrationRequest {
            registration_token: handoff_code,
            nickname: "google_only".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("google registration should succeed")
}

fn state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        server_store: Arc::new(InMemoryServerStore::default()),
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        voice_presence_store: Arc::new(
            crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore::default(),
        ),
        realtime_hub: Arc::new(RealtimeHub::default()),
        auth_keys: AuthKeys::generate_for_tests(),
        access_token_lifetime_minutes: 15,
        refresh_token_lifetime_days: 30,
        google_oauth_client_id: Some("test-google-client".to_owned()),
        google_oauth_client_secret: Some("test-google-secret".to_owned()),
        google_oauth_redirect_uri: Some(
            "http://localhost/api/auth/oauth/google/callback".to_owned(),
        ),
        cheenhub_client_base_url: "http://localhost".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
    }
}
