//! Проверки аутентификации HTTP-операций личных сообщений.

use std::sync::Arc;

use super::*;
use crate::features::auth::email::tests::TestAuthMailer;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::images::infrastructure::InMemoryImageStore;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::{
    InMemoryChatAttachmentObjectStore, InMemoryTextChatStore,
};
use crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;
use cheenhub_contracts::rest::RegisterRequest;

#[tokio::test]
async fn missing_or_malformed_token_cannot_list_conversations() {
    let state = state();

    for authorization in [
        None,
        Some("Basic invalid"),
        Some("Bearer "),
        Some("Bearer invalid"),
    ] {
        let mut headers = HeaderMap::new();
        if let Some(value) = authorization {
            headers.insert(axum::http::header::AUTHORIZATION, value.parse().unwrap());
        }

        let result = list_dm_conversations(State(state.clone()), headers).await;

        assert!(
            matches!(result, Err(SocialError::Unauthorized(_))),
            "{authorization:?}"
        );
    }
}

#[tokio::test]
async fn missing_token_rejects_every_conversation_operation() {
    let state = state();
    let conversation_id = Uuid::new_v4().to_string();
    let message_id = Uuid::new_v4().to_string();

    let results = [
        open_dm_conversation(
            State(state.clone()),
            HeaderMap::new(),
            Json(OpenDmConversationRequest {
                friend_user_id: Uuid::new_v4().to_string(),
            }),
        )
        .await
        .map(|_| ()),
        list_dm_messages(
            State(state.clone()),
            HeaderMap::new(),
            Path(conversation_id.clone()),
            Query(DmMessagesQuery {
                before_message_id: None,
            }),
        )
        .await
        .map(|_| ()),
        send_dm_message(
            State(state.clone()),
            HeaderMap::new(),
            Path(conversation_id.clone()),
            Json(SendDmMessageRequest {
                body: "Недоступно без сессии".to_owned(),
                image_id: None,
            }),
        )
        .await
        .map(|_| ()),
        edit_dm_message(
            State(state.clone()),
            HeaderMap::new(),
            Path((conversation_id.clone(), message_id.clone())),
            Json(EditDmMessageRequest {
                body: "Недоступно без сессии".to_owned(),
            }),
        )
        .await
        .map(|_| ()),
        upload_dm_image(
            State(state.clone()),
            HeaderMap::new(),
            Path(conversation_id.clone()),
            Bytes::new(),
        )
        .await
        .map(|_| ()),
        dm_image(
            State(state.clone()),
            HeaderMap::new(),
            Path((conversation_id.clone(), Uuid::new_v4().to_string())),
        )
        .await
        .map(|_| ()),
        mark_dm_conversation_read(
            State(state),
            HeaderMap::new(),
            Path(conversation_id),
            Json(MarkDmConversationReadRequest {
                last_read_message_id: message_id,
            }),
        )
        .await
        .map(|_| ()),
    ];

    for (operation, result) in [
        "open",
        "history",
        "send",
        "edit",
        "upload_image",
        "load_image",
        "mark_read",
    ]
    .into_iter()
    .zip(results)
    {
        assert!(
            matches!(result, Err(SocialError::Unauthorized(_))),
            "{operation}"
        );
    }
}

#[tokio::test]
async fn valid_session_can_list_conversations_but_revoked_session_cannot() {
    let state = state();
    let auth = crate::features::auth::application::register(
        &state,
        RegisterRequest {
            nickname: "dm_http".to_owned(),
            email: "dm-http@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed");
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        format!("Bearer {}", auth.access_token).parse().unwrap(),
    );
    let user_id = Uuid::parse_str(&auth.user.id).unwrap();

    let response = list_dm_conversations(State(state.clone()), headers.clone())
        .await
        .expect("active session should work");
    assert!(response.0.conversations.is_empty());
    state
        .auth_store
        .revoke_user_sessions(&user_id, chrono::Utc::now())
        .await
        .expect("sessions should revoke");
    let result = list_dm_conversations(State(state), headers).await;

    assert!(matches!(result, Err(SocialError::Unauthorized(_))));
}

fn state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(TestAuthMailer::default()),
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
        chat_attachment_object_store: Arc::new(InMemoryChatAttachmentObjectStore::new(
            "test-chat-images",
        )),
        image_store: Arc::new(InMemoryImageStore::default()),
        push_notifications: Arc::new(
            crate::features::push_notifications::application::PushNotifications::disabled(
                Arc::new(InMemoryAuthStore::default()),
            ),
        ),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
        voice_presence_store: Arc::new(InMemoryVoicePresenceStore::default()),
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
    }
}
