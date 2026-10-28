//! Проверки команд ЛС через общий realtime-обработчик.

mod direct_messages;

use super::*;
use crate::features::auth::email::tests::TestAuthMailer;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::images::infrastructure::InMemoryImageStore;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::application;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::{
    InMemoryChatAttachmentObjectStore, InMemoryTextChatStore,
};
use crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore;
use crate::realtime::WebSocketOutbound;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;
use cheenhub_contracts::rest::{
    AuthResponse, OpenDmConversationRequest, RegisterRequest, SendDmMessageRequest,
    SendFriendRequestRequest,
};
use std::sync::Arc;
use tokio::sync::mpsc;

struct Pair {
    state: AppState,
    alice: AuthUser,
    bob: AuthUser,
    conversation_id: String,
}

async fn pair() -> Pair {
    let state = state();
    let alice = register(&state, "alice_rt", "alice-rt@example.com").await;
    let bob = register(&state, "bob_rt", "bob-rt@example.com").await;
    let request = application::send_friend_request(
        &state,
        &alice.access_token,
        SendFriendRequestRequest {
            recipient_user_id: bob.user.id.clone(),
        },
    )
    .await
    .unwrap();
    application::accept_friend_request(&state, &bob.access_token, request.request.id)
        .await
        .unwrap();
    let conversation = application::open_dm_conversation(
        &state,
        &id(&alice.user),
        OpenDmConversationRequest {
            friend_user_id: bob.user.id.clone(),
        },
    )
    .await
    .unwrap()
    .conversation;
    Pair {
        state,
        alice: alice.user,
        bob: bob.user,
        conversation_id: conversation.id,
    }
}

async fn register(state: &AppState, nickname: &str, email: &str) -> AuthResponse {
    crate::features::auth::application::register(
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
    .unwrap()
}

fn id(user: &AuthUser) -> Uuid {
    Uuid::parse_str(&user.id).unwrap()
}

async fn command(
    state: &AppState,
    user: &AuthUser,
    kind: &str,
    payload: serde_json::Value,
    request_id: Option<Uuid>,
) -> RealtimeEnvelope {
    let envelope: RealtimeEnvelope = serde_json::from_value(serde_json::json!({
        "module": "social", "kind": kind, "request_id": request_id, "payload": payload,
    }))
    .unwrap();
    let (sender, mut receiver) = mpsc::channel(8);
    handle(
        state,
        user,
        &id(user),
        Uuid::new_v4(),
        &EnvelopeSink::websocket(sender),
        envelope,
    )
    .await
    .expect("command should produce a response");
    let WebSocketOutbound::Envelope(response) =
        receiver.recv().await.expect("response should arrive")
    else {
        panic!("expected response envelope");
    };
    assert_eq!(response.request_id, request_id);
    assert!(response.has_matching_module_kind());
    response
}

async fn send(
    pair: &Pair,
    user: &AuthUser,
    body: &str,
) -> cheenhub_contracts::rest::DmMessageSummary {
    application::send_dm_message(
        &pair.state,
        &id(user),
        pair.conversation_id.clone(),
        SendDmMessageRequest {
            body: body.to_owned(),
            image_id: None,
        },
    )
    .await
    .unwrap()
    .message
}

fn response<T: serde::de::DeserializeOwned>(envelope: RealtimeEnvelope, kind: &str) -> T {
    assert_eq!(serde_json::to_value(envelope.kind).unwrap(), kind);
    serde_json::from_value(envelope.payload).unwrap()
}

fn rejection(envelope: RealtimeEnvelope, code: RejectionCode) {
    let rejected: cheenhub_contracts::realtime::Rejected = response(envelope, "rejected");
    assert_eq!(rejected.code, code);
}

#[tokio::test]
async fn presence_updates_reach_friends_but_not_unrelated_users() {
    use crate::features::social::presence::notify_presence_change;
    use crate::realtime::hub::{SessionActivity, SessionActivityChange};

    let pair = pair().await;
    let unrelated = register(&pair.state, "unrelated_rt", "unrelated-rt@example.com").await;
    let (friend_sender, mut friend_receiver) = mpsc::channel(4);
    let (unrelated_sender, mut unrelated_receiver) = mpsc::channel(4);
    for (user, sender) in [
        (&pair.bob, friend_sender),
        (&unrelated.user, unrelated_sender),
    ] {
        pair.state
            .realtime_hub
            .register_stream(
                Uuid::new_v4(),
                RealtimeModule::Social,
                id(user),
                EnvelopeSink::websocket(sender),
            )
            .await;
    }

    notify_presence_change(
        &pair.state,
        SessionActivityChange {
            user_id: id(&pair.alice),
            activity: SessionActivity {
                online: true,
                last_seen_at: None,
            },
        },
    )
    .await;

    let WebSocketOutbound::PreparedEnvelope(text) = friend_receiver
        .try_recv()
        .expect("friend should receive presence")
    else {
        panic!("expected a prepared presence envelope");
    };
    let envelope: RealtimeEnvelope = serde_json::from_str(text.as_str()).unwrap();
    assert_eq!(
        envelope.kind,
        RealtimeKind::Social(SocialKind::PresenceChanged)
    );
    let payload: cheenhub_contracts::realtime::PresenceChanged =
        serde_json::from_value(envelope.payload).unwrap();
    assert_eq!(payload.user_id, pair.alice.id);
    assert_eq!(
        payload.presence,
        cheenhub_contracts::presence::UserPresence::Online
    );
    assert!(unrelated_receiver.try_recv().is_err());
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
