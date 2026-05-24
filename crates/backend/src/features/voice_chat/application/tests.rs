//! Voice chat application tests.

use std::sync::Arc;

use cheenhub_contracts::realtime::{JoinVoiceRoom, LeaveVoiceRoom, ListServerVoiceRooms};
use cheenhub_contracts::rest::{RegisterRequest, ServerRoomKind};

use super::{VoiceChatApplicationError, join_room, leave_room, list_server_voice_rooms};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

mod nickname;

pub(super) fn state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(crate::features::auth::email::tests::TestAuthMailer::default()),
        server_store: Arc::new(InMemoryServerStore::default()),
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
        voice_presence_store: Arc::new(InMemoryVoicePresenceStore::default()),
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
        password_reset_token_lifetime_minutes: 30,
        cheenhub_api_base_url: "http://localhost/api".to_owned(),
    }
}

pub(super) async fn registered_user(
    state: &AppState,
) -> (cheenhub_contracts::rest::AuthUser, uuid::Uuid) {
    let auth = auth_application::register(
        state,
        RegisterRequest {
            nickname: "voice_owner".to_owned(),
            email: "voice-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should be uuid");

    (auth.user, user_id)
}

pub(super) async fn create_room(
    state: &AppState,
    user_id: &uuid::Uuid,
    room_name: &str,
    kind: ServerRoomKind,
) -> (String, String) {
    let server = state
        .server_store
        .insert_server(user_id, "Voice Server".to_owned())
        .await
        .expect("server should insert");
    state
        .server_store
        .insert_server_member(&server.id, user_id)
        .await
        .expect("member should insert");
    let room = state
        .server_store
        .insert_server_room(&server.id, room_name.to_owned(), kind)
        .await
        .expect("room should insert");

    (server.id.to_string(), room.id.to_string())
}

#[tokio::test]
async fn user_can_join_and_leave_voice_room() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;

    let snapshot = join_room(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .expect("join should succeed");

    assert_eq!(snapshot.participants.len(), 1);
    assert_eq!(snapshot.participants[0].nickname, "voice_owner");

    let snapshot = leave_room(
        &state,
        stream_id,
        &user_id,
        LeaveVoiceRoom { server_id, room_id },
    )
    .await
    .expect("leave should succeed");

    assert!(snapshot.participants.is_empty());
}

#[tokio::test]
async fn text_room_rejects_voice_join() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (server_id, room_id) = create_room(&state, &user_id, "text", ServerRoomKind::Text).await;

    let error = join_room(
        &state,
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom { server_id, room_id },
    )
    .await
    .expect_err("text rooms should reject voice presence");

    assert!(matches!(error, VoiceChatApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn user_can_leave_after_room_becomes_text_only() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;

    join_room(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .expect("join should succeed");

    let server_uuid = uuid::Uuid::parse_str(&server_id).expect("server id");
    let room_uuid = uuid::Uuid::parse_str(&room_id).expect("room id");
    state
        .server_store
        .update_server_room(
            &server_uuid,
            &room_uuid,
            "voice".to_owned(),
            ServerRoomKind::Text,
        )
        .await
        .expect("room update should succeed");

    let snapshot = leave_room(
        &state,
        stream_id,
        &user_id,
        LeaveVoiceRoom { server_id, room_id },
    )
    .await
    .expect("leave should remove existing stale presence");

    assert!(snapshot.participants.is_empty());
}

#[tokio::test]
async fn joining_new_room_replaces_previous_presence() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let (first_server_id, first_room_id) =
        create_room(&state, &user_id, "first", ServerRoomKind::Voice).await;
    let (second_server_id, second_room_id) =
        create_room(&state, &user_id, "second", ServerRoomKind::TextAndVoice).await;

    join_room(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: first_server_id.clone(),
            room_id: first_room_id.clone(),
        },
    )
    .await
    .expect("first join should succeed");
    let snapshot = join_room(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: second_server_id.clone(),
            room_id: second_room_id.clone(),
        },
    )
    .await
    .expect("second join should succeed");

    assert_eq!(snapshot.server_id, second_server_id);
    assert_eq!(snapshot.room_id, second_room_id);
    assert_eq!(snapshot.participants.len(), 1);

    let first_server = uuid::Uuid::parse_str(&first_server_id).expect("server id");
    let first_room = uuid::Uuid::parse_str(&first_room_id).expect("room id");
    assert!(
        state
            .voice_presence_store
            .room_participants(&first_server, &first_room)
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn member_can_list_active_voice_room_participants_for_server() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;

    join_room(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .expect("join should succeed");

    let snapshot = list_server_voice_rooms(
        &state,
        &user_id,
        ListServerVoiceRooms {
            server_id: server_id.clone(),
        },
    )
    .await
    .expect("server voice rooms should list");

    assert_eq!(snapshot.server_id, server_id);
    assert_eq!(snapshot.rooms.len(), 1);
    assert_eq!(snapshot.rooms[0].room_id, room_id);
    assert_eq!(snapshot.rooms[0].participants[0].nickname, "voice_owner");
}
