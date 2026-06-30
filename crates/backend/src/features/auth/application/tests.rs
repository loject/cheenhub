//! Тесты приложения аутентификации.

use std::sync::Arc;

use cheenhub_contracts::rest::{
    LoginRequest, OAuthRegistrationRequest, PasswordResetConfirmRequest, PasswordResetRequest,
    RegisterRequest,
};
use chrono::{Duration, Utc};

use super::{
    confirm_password_reset, login, me, register, register_with_google_oauth, request_password_reset,
};
use crate::features::auth::email::tests::TestAuthMailer;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::{keys::AuthKeys, refresh_token};
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

mod avatar;
mod nickname;
mod oauth;
mod password;
mod sessions;

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

pub(super) async fn registered_user(
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

pub(super) async fn google_only_user(state: &AppState) -> cheenhub_contracts::rest::AuthResponse {
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
        None,
    )
    .await
    .expect("google registration should succeed")
}

pub(super) fn state() -> AppState {
    state_with_mailer().0
}

pub(super) fn state_with_mailer() -> (AppState, Arc<TestAuthMailer>) {
    let mailer = Arc::new(TestAuthMailer::default());
    let state = AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: mailer.clone(),
        server_store: Arc::new(InMemoryServerStore::default()),
        social_store: Arc::new(InMemorySocialStore::default()),
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        chat_attachment_object_store: Arc::new(
            crate::features::text_chat::infrastructure::InMemoryChatAttachmentObjectStore::new(
                "test-chat-images",
            ),
        ),
        image_store: Arc::new(
            crate::features::images::infrastructure::InMemoryImageStore::default(),
        ),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
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
        cheenhub_api_base_url: "http://localhost/api".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
        password_reset_token_lifetime_minutes: 30,
    };

    (state, mailer)
}

fn reset_token_from_mailer(mailer: &TestAuthMailer) -> String {
    let sent = mailer.sent();
    sent.last()
        .and_then(|email| email.reset_url.split("token=").nth(1))
        .expect("reset token should be present")
        .to_owned()
}
