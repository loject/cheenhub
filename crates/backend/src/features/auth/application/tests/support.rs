//! Общие помощники тестов приложения аутентификации.

use std::sync::Arc;

use cheenhub_contracts::rest::{AuthResponse, OAuthRegistrationRequest, RegisterRequest};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::features::auth::application::{register, register_with_google_oauth};
use crate::features::auth::email::tests::TestAuthMailer;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::{jwt, keys::AuthKeys, refresh_token};
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

/// Регистрирует тестовый realtime-транспорт для auth-ответа.
///
/// Нужен сценариям, которые проверяют завершение активных сессий.
pub(super) async fn register_test_session(
    state: &AppState,
    auth: &AuthResponse,
) -> tokio::sync::watch::Receiver<Option<crate::realtime::hub::DisconnectReason>> {
    let claims = jwt::verify_access_token(&state.auth_keys, &auth.access_token)
        .expect("test access token should be valid");
    let user_id = Uuid::parse_str(&auth.user.id).expect("test user id should be a uuid");
    let auth_session_id =
        Uuid::parse_str(&claims.session_id).expect("test auth session id should be a uuid");
    state
        .realtime_hub
        .register_test_session(user_id, auth_session_id)
        .await
}

pub(super) async fn registered_user(state: &AppState, nickname: &str, email: &str) -> AuthResponse {
    register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed")
}

pub(super) async fn google_only_user(state: &AppState) -> AuthResponse {
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
            accepts_terms: true,
            accepts_personal_data: true,
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
        host_settings_store: Arc::new(
            crate::features::host_settings::infrastructure::InMemoryHostSettingsStore::default(),
        ),
        host_metrics: Arc::new(
            crate::features::host_settings::metrics_monitor::HostMetricsMonitor::disabled(),
        ),
        host_logs: Arc::new(crate::telemetry::HostLogHub::default()),
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
        push_notifications: Arc::new(
            crate::features::push_notifications::application::PushNotifications::disabled(
                Arc::new(InMemoryAuthStore::default()),
            ),
        ),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
        voice_presence_store: Arc::new(
            crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore::default(),
        ),
        direct_call_store: Arc::new(
            crate::features::voice_chat::infrastructure::InMemoryDirectCallStore::default(),
        ),
        typing_store: Arc::new(crate::features::typing::InMemoryTypingStore::default()),
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

pub(super) fn reset_token_from_mailer(mailer: &TestAuthMailer) -> String {
    let sent = mailer.sent();
    sent.last()
        .and_then(|email| email.reset_url.split("token=").nth(1))
        .expect("reset token should be present")
        .to_owned()
}
