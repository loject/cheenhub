//! Проверки сводной статистики CheenHub для владельца хоста.

use std::sync::Arc;

use cheenhub_contracts::rest::{
    OpenDmConversationRequest, RegisterRequest, SendDmMessageRequest, SendFriendRequestRequest,
};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::stats;
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::host_settings::application::HostSettingsError;
use crate::features::host_settings::infrastructure::InMemoryHostSettingsStore;
use crate::features::images::infrastructure::InMemoryImageStore;
use crate::features::push_notifications::application::PushNotifications;
use crate::features::servers::domain::ServerRoomWriteAccess;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::domain::TextMessage;
use crate::features::text_chat::infrastructure::{
    InMemoryChatAttachmentObjectStore, InMemoryTextChatStore,
};
use crate::features::voice_chat::infrastructure::{
    InMemoryDirectCallStore, InMemoryVoicePresenceStore,
};
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

fn state() -> AppState {
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
            "stats-test-images",
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
        google_oauth_client_id: Some("stats-test-client".to_owned()),
        google_oauth_client_secret: Some("stats-test-secret".to_owned()),
        google_oauth_redirect_uri: Some(
            "http://localhost/api/auth/oauth/google/callback".to_owned(),
        ),
        typing_store: Arc::new(crate::features::typing::InMemoryTypingStore::default()),
        cheenhub_client_base_url: "http://localhost".to_owned(),
        cheenhub_api_base_url: "http://localhost/api".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
        password_reset_token_lifetime_minutes: 30,
    }
}

async fn register(
    state: &AppState,
    nickname: &str,
    email: &str,
) -> cheenhub_contracts::rest::AuthResponse {
    auth_application::register(
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

async fn owner_state() -> (AppState, cheenhub_contracts::rest::AuthResponse, Uuid) {
    let state = state();
    let auth = register(&state, "stats_owner", "stats-owner@example.com").await;
    let owner_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let state = AppState {
        host_settings_store: Arc::new(InMemoryHostSettingsStore::with_owner(owner_id)),
        ..state
    };
    (state, auth, owner_id)
}

/// Создаёт сервер с одной комнатой и возвращает их идентификаторы.
async fn server_with_room(state: &AppState, owner_id: Uuid) -> (Uuid, Uuid) {
    let server = state
        .server_store
        .insert_server(&owner_id, "Stats Server".to_owned())
        .await
        .expect("server should insert");
    let room = state
        .server_store
        .insert_server_room(
            &server.id,
            "general".to_owned(),
            cheenhub_contracts::rest::ServerRoomKind::TextAndVoice,
            ServerRoomWriteAccess::all_members(),
        )
        .await
        .expect("room should insert");

    (server.id, room.id)
}

/// Сохраняет текстовое сообщение с указанным временем отправки.
async fn insert_message(
    state: &AppState,
    server_id: Uuid,
    room_id: Uuid,
    author_user_id: Uuid,
    created_at: DateTime<Utc>,
) {
    state
        .text_chat_store
        .insert_text_message(TextMessage {
            id: Uuid::new_v4(),
            server_id,
            room_id,
            author_user_id,
            author_nickname: "stats_owner".to_owned(),
            body: "message".to_owned(),
            attachments: Vec::new(),
            created_at,
            deleted_at: None,
            deleted_by_user_id: None,
        })
        .await
        .expect("message should insert");
}

/// Делает двух пользователей друзьями по настоящему сценарию social.
async fn become_friends(
    state: &AppState,
    sender: &cheenhub_contracts::rest::AuthResponse,
    recipient: &cheenhub_contracts::rest::AuthResponse,
) {
    let request = social::send_friend_request(
        state,
        &sender.access_token,
        SendFriendRequestRequest {
            recipient_user_id: recipient.user.id.clone(),
        },
    )
    .await
    .expect("friend request should be sent");
    social::accept_friend_request(state, &recipient.access_token, request.request.id.clone())
        .await
        .expect("friend request should be accepted");
}

/// Отправляет личное сообщение другу по настоящему сценарию social.
///
/// Тест идёт через application-слой, поэтому личные сообщения попадают в
/// статистику тем же путём, что и в реальном приложении.
async fn send_direct_message(
    state: &AppState,
    sender: &cheenhub_contracts::rest::AuthResponse,
    friend: &cheenhub_contracts::rest::AuthResponse,
) {
    let conversation = social::open_dm_conversation(
        state,
        &sender.access_token,
        OpenDmConversationRequest {
            friend_user_id: friend.user.id.clone(),
        },
    )
    .await
    .expect("conversation should be opened");

    social::send_dm_message(
        state,
        &sender.access_token,
        conversation.conversation.id.clone(),
        SendDmMessageRequest {
            body: "personal message".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("direct message should be sent");
}

#[tokio::test]
async fn owner_sees_users_servers_rooms_and_message_totals() {
    let (state, owner, owner_id) = owner_state().await;
    register(&state, "stats_member", "stats-member@example.com").await;
    let (server_id, room_id) = server_with_room(&state, owner_id).await;

    insert_message(&state, server_id, room_id, owner_id, Utc::now()).await;
    insert_message(&state, server_id, room_id, owner_id, Utc::now()).await;

    let snapshot = stats(&state, &owner.access_token)
        .await
        .expect("owner should read host statistics");

    assert_eq!(snapshot.users_total, 2, "считаются все пользователи хоста");
    assert_eq!(snapshot.servers_total, 1);
    assert_eq!(snapshot.rooms_total, 1);
    assert_eq!(snapshot.room_messages_total, 2);
    assert_eq!(snapshot.direct_messages_total, 0);
}

#[tokio::test]
async fn direct_messages_are_counted_next_to_room_messages() {
    let (state, owner, owner_id) = owner_state().await;
    let member = register(&state, "stats_member", "stats-member@example.com").await;
    let (server_id, room_id) = server_with_room(&state, owner_id).await;
    insert_message(&state, server_id, room_id, owner_id, Utc::now()).await;
    become_friends(&state, &owner, &member).await;

    send_direct_message(&state, &owner, &member).await;
    send_direct_message(&state, &member, &owner).await;

    let snapshot = stats(&state, &owner.access_token)
        .await
        .expect("owner should read host statistics");

    assert_eq!(snapshot.room_messages_total, 1);
    assert_eq!(
        snapshot.direct_messages_total, 2,
        "личные сообщения считаются вместе с сообщениями комнат"
    );
    assert_eq!(snapshot.direct_messages_per_minute.len(), 1);
    assert_eq!(snapshot.direct_messages_per_minute[0].messages, 2);
    assert_eq!(
        snapshot.room_messages_per_minute.len(),
        1,
        "комнаты и личные диалоги попадают в разные серии"
    );
}

#[tokio::test]
async fn soft_deleted_message_stays_in_totals_and_minute_chart() {
    let (state, owner, owner_id) = owner_state().await;
    let (server_id, room_id) = server_with_room(&state, owner_id).await;
    insert_message(&state, server_id, room_id, owner_id, Utc::now()).await;
    let message_id = state
        .text_chat_store
        .room_message_page(&room_id, None)
        .await
        .expect("room history should load")
        .messages
        .first()
        .expect("inserted message is in history")
        .id;

    state
        .text_chat_store
        .soft_delete_message(&server_id, &room_id, &message_id, &owner_id, true)
        .await
        .expect("own message should be deleted")
        .expect("message is deleted");

    let snapshot = stats(&state, &owner.access_token)
        .await
        .expect("owner should read host statistics");
    assert_eq!(
        snapshot.room_messages_total, 1,
        "удалённое сообщение уже было отправлено и остаётся в статистике"
    );
    assert_eq!(snapshot.room_messages_per_minute[0].messages, 1);
}

#[tokio::test]
async fn regular_user_is_denied_host_statistics() {
    let (state, _owner, _owner_id) = owner_state().await;
    let regular = register(&state, "stats_regular", "stats-regular@example.com").await;

    let denied = stats(&state, &regular.access_token).await;

    assert!(
        matches!(denied, Err(HostSettingsError::Forbidden(_))),
        "обычному пользователю сводная статистика недоступна"
    );
}

#[tokio::test]
async fn messages_per_minute_cover_only_the_last_day() {
    let (state, owner, owner_id) = owner_state().await;
    let (server_id, room_id) = server_with_room(&state, owner_id).await;
    let now = Utc::now();
    // Минута целиком в прошлом: обе отметки внутри неё гарантированно попадают
    // в окно суток, которое сервер отсчитывает от своего `now`.
    let closed_minute = DateTime::from_timestamp(now.timestamp() - now.timestamp() % 60 - 120, 0)
        .expect("valid minute");

    insert_message(&state, server_id, room_id, owner_id, closed_minute).await;
    insert_message(
        &state,
        server_id,
        room_id,
        owner_id,
        closed_minute + Duration::seconds(30),
    )
    .await;
    insert_message(
        &state,
        server_id,
        room_id,
        owner_id,
        closed_minute - Duration::hours(25),
    )
    .await;

    let snapshot = stats(&state, &owner.access_token)
        .await
        .expect("owner should read host statistics");

    assert_eq!(
        snapshot.room_messages_total, 3,
        "общий счётчик учитывает сообщения за всё время"
    );
    assert_eq!(
        snapshot.room_messages_per_minute.len(),
        1,
        "в график попадает только закрытая минута"
    );
    assert_eq!(snapshot.room_messages_per_minute[0].messages, 2);
    assert_eq!(
        snapshot.room_messages_per_minute[0].minute_unix_ms,
        closed_minute.timestamp_millis(),
        "минута выровнена по границе минуты UTC"
    );
    assert!(snapshot.messages_window_end_unix_ms >= now.timestamp_millis());
    assert!(snapshot.messages_window_end_unix_ms <= Utc::now().timestamp_millis());
}
