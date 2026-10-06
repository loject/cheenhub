//! Проверка отзыва доступа у уже открытого потока серверных логов.

use std::{sync::Arc, time::Duration};

use cheenhub_contracts::rest::{HostLogStreamMessage, RegisterRequest};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use uuid::Uuid;

use super::*;
use crate::features::auth::{
    application as auth_application, infrastructure::InMemoryAuthStore, security::keys::AuthKeys,
};
use crate::features::host_settings::{
    domain::HostOwner, infrastructure::InMemoryHostSettingsStore,
};
use crate::features::images::infrastructure::InMemoryImageStore;
use crate::features::push_notifications::application::PushNotifications;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::{
    InMemoryChatAttachmentObjectStore, InMemoryTextChatStore,
};
use crate::features::voice_chat::infrastructure::{
    InMemoryDirectCallStore, InMemoryVoicePresenceStore,
};
use crate::realtime::hub::RealtimeHub;

use crate::features::host_settings::infrastructure::HostSettingsStore;

fn base_state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(crate::features::auth::email::tests::TestAuthMailer::default()),
        host_settings_store: Arc::new(InMemoryHostSettingsStore::default()),
        host_metrics: Arc::new(
            crate::features::host_settings::metrics_monitor::HostMetricsMonitor::disabled(),
        ),
        host_logs: Arc::new(crate::telemetry::HostLogHub::default()),
        server_store: Arc::new(InMemoryServerStore::default()),
        social_store: Arc::new(InMemorySocialStore::default()),
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        chat_attachment_object_store: Arc::new(InMemoryChatAttachmentObjectStore::new(
            "owners-test-images",
        )),
        image_store: Arc::new(InMemoryImageStore::default()),
        push_notifications: Arc::new(PushNotifications::disabled(Arc::new(
            InMemoryAuthStore::default(),
        ))),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
        voice_presence_store: Arc::new(InMemoryVoicePresenceStore::default()),
        direct_call_store: Arc::new(InMemoryDirectCallStore::default()),
        realtime_hub: Arc::new(RealtimeHub::default()),
        auth_keys: AuthKeys::generate_for_tests(),
        access_token_lifetime_minutes: 15,
        refresh_token_lifetime_days: 30,
        google_oauth_client_id: None,
        google_oauth_client_secret: None,
        google_oauth_redirect_uri: None,
        typing_store: Arc::new(crate::features::typing::InMemoryTypingStore::default()),
        cheenhub_client_base_url: "http://localhost".to_owned(),
        cheenhub_api_base_url: "http://localhost/api".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
        password_reset_token_lifetime_minutes: 30,
    }
}

struct RunningServer(tokio::task::JoinHandle<()>);

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

type ClientSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn open_log_stream(url: &str, token: &str) -> ClientSocket {
    let (mut socket, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket connection");
    socket
        .send(Message::Text(
            serde_json::json!({ "access_token": token })
                .to_string()
                .into(),
        ))
        .await
        .expect("stream authentication");
    assert!(matches!(
        receive_message(&mut socket).await,
        HostLogStreamMessage::Snapshot { .. }
    ));
    socket
}

async fn receive_message(socket: &mut ClientSocket) -> HostLogStreamMessage {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match socket
                .next()
                .await
                .expect("stream must remain open")
                .expect("websocket message")
            {
                Message::Text(text) => return serde_json::from_str(&text).expect("log contract"),
                Message::Ping(payload) => socket
                    .send(Message::Pong(payload))
                    .await
                    .expect("heartbeat pong"),
                message => panic!("unexpected websocket message: {message:?}"),
            }
        }
    })
    .await
    .expect("log stream response deadline")
}

async fn receive_after_revocation(socket: &mut ClientSocket) -> HostLogStreamMessage {
    loop {
        let message = receive_message(socket).await;
        // До отзыва socket мог получить запись об открытии другого потока.
        // Проверяем только данные, созданные после завершения отзыва.
        if matches!(&message, HostLogStreamMessage::Entry { entry }
            if entry.message != "log emitted after owner revocation")
        {
            continue;
        }
        return message;
    }
}

#[tokio::test]
#[ignore = "Требуется разрешение на локальные TCP-сокеты; тест поднимает изолированный сервер на случайном порту."]
async fn revoked_owner_stream_rejects_new_logs_while_remaining_owner_keeps_access() {
    let state = base_state();
    let mut accounts = Vec::new();
    for (nickname, email) in [
        ("logs_owner", "logs-owner@example.com"),
        ("logs_second", "logs-second@example.com"),
    ] {
        accounts.push(
            auth_application::register(
                &state,
                RegisterRequest {
                    nickname: nickname.to_owned(),
                    email: email.to_owned(),
                    password: "password123".to_owned(),
                    accepts_terms: true,
                    accepts_personal_data: true,
                },
            )
            .await
            .expect("register owner"),
        );
    }
    let first_id = Uuid::parse_str(&accounts[0].user.id).expect("first user id");
    let second_id = Uuid::parse_str(&accounts[1].user.id).expect("second user id");
    let store = Arc::new(InMemoryHostSettingsStore::with_owner(first_id));
    store
        .grant_host_owner(HostOwner {
            user_id: second_id,
            granted_at: chrono::Utc::now(),
            granted_by_user_id: Some(first_id),
        })
        .await
        .expect("grant second owner");
    let state = AppState {
        host_settings_store: store.clone(),
        host_logs: crate::telemetry::init("info").expect("isolated log subscriber"),
        ..state
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("isolated TCP listener");
    let address = listener.local_addr().expect("listener address");
    let router = routes().with_state(state);
    let _server = RunningServer(tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve log websocket");
    }));
    let url = format!("ws://{address}/logs/ws");
    let mut revoked = open_log_stream(&url, &accounts[1].access_token).await;
    let mut remaining = open_log_stream(&url, &accounts[0].access_token).await;

    store
        .revoke_host_owner(second_id)
        .await
        .expect("revoke owner");
    tracing::info!("log emitted after owner revocation");

    assert!(
        matches!(
            receive_after_revocation(&mut revoked).await,
            HostLogStreamMessage::Error {
                retryable: false,
                ..
            }
        ),
        "revoked owner must receive an access error instead of new server logs"
    );
    assert!(
        matches!(
            receive_after_revocation(&mut remaining).await,
            HostLogStreamMessage::Entry { .. }
        ),
        "remaining owner keeps the live log stream"
    );
}
