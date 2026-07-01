//! Voice chat application tests.

use std::sync::Arc;

use cheenhub_contracts::realtime::{
    JoinDirectMessageVoiceRoom, JoinVoiceRoom, LeaveDirectMessageVoiceRoom, LeaveVoiceRoom,
    ListDirectMessageVoiceRooms, ListServerVoiceRooms, StopVoiceVideoStream,
    VoiceVideoStreamSource,
};
use cheenhub_contracts::rest::{RegisterRequest, ServerRoomKind};

use super::{
    VoiceChatApplicationError, join_direct_message_room, join_room, leave_direct_message_room,
    leave_room, list_direct_message_voice_rooms, list_server_voice_rooms, stop_video_stream,
};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::social::{accept_friend_request, open_dm_conversation, send_friend_request};
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

async fn registered_user_with_email(
    state: &AppState,
    nickname: &str,
    email: &str,
) -> (cheenhub_contracts::rest::AuthResponse, uuid::Uuid) {
    let auth = auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should be uuid");

    (auth, user_id)
}

async fn direct_message_conversation(
    state: &AppState,
) -> (
    cheenhub_contracts::rest::AuthUser,
    uuid::Uuid,
    cheenhub_contracts::rest::AuthUser,
    uuid::Uuid,
    String,
) {
    let (alice_auth, alice_id) =
        registered_user_with_email(state, "voice_alice", "voice-alice@example.com").await;
    let (bob_auth, bob_id) =
        registered_user_with_email(state, "voice_bob", "voice-bob@example.com").await;
    let request = send_friend_request(
        state,
        &alice_auth.access_token,
        cheenhub_contracts::rest::SendFriendRequestRequest {
            recipient_user_id: bob_auth.user.id.clone(),
        },
    )
    .await
    .expect("friend request should send");
    accept_friend_request(state, &bob_auth.access_token, request.request.id)
        .await
        .expect("friend request should accept");
    let conversation = open_dm_conversation(
        state,
        &alice_auth.access_token,
        cheenhub_contracts::rest::OpenDmConversationRequest {
            friend_user_id: bob_auth.user.id.clone(),
        },
    )
    .await
    .expect("conversation should open")
    .conversation;

    (
        alice_auth.user,
        alice_id,
        bob_auth.user,
        bob_id,
        conversation.id,
    )
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
            .room_participants(
                crate::features::voice_chat::infrastructure::VoicePresenceTargetKind::Server,
                &first_server,
                &first_room,
            )
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn friends_can_join_and_leave_direct_message_voice_room() {
    let state = state();
    let (alice, alice_id, bob, bob_id, conversation_id) = direct_message_conversation(&state).await;
    let alice_stream_id = uuid::Uuid::new_v4();
    let bob_stream_id = uuid::Uuid::new_v4();

    let first_snapshot = join_direct_message_room(
        &state,
        alice_stream_id,
        uuid::Uuid::new_v4(),
        &alice,
        &alice_id,
        JoinDirectMessageVoiceRoom {
            conversation_id: conversation_id.clone(),
        },
    )
    .await
    .expect("alice should join dm voice");
    assert_eq!(first_snapshot.room_id, conversation_id);
    assert_eq!(first_snapshot.participants.len(), 1);

    let second_snapshot = join_direct_message_room(
        &state,
        bob_stream_id,
        uuid::Uuid::new_v4(),
        &bob,
        &bob_id,
        JoinDirectMessageVoiceRoom {
            conversation_id: conversation_id.clone(),
        },
    )
    .await
    .expect("bob should join dm voice");
    assert_eq!(second_snapshot.participants.len(), 2);

    let active = list_direct_message_voice_rooms(&state, &alice_id, ListDirectMessageVoiceRooms)
        .await
        .expect("active dm voice rooms should list");
    assert_eq!(active.rooms.len(), 1);
    assert_eq!(active.rooms[0].room_id, conversation_id);

    let left_snapshot = leave_direct_message_room(
        &state,
        alice_stream_id,
        &alice_id,
        LeaveDirectMessageVoiceRoom { conversation_id },
    )
    .await
    .expect("alice should leave dm voice");
    assert_eq!(left_snapshot.participants.len(), 1);
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

#[tokio::test]
async fn joined_user_can_stop_video_stream() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let session_id = uuid::Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;

    join_room(
        &state,
        stream_id,
        session_id,
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .expect("join should succeed");

    stop_video_stream(
        &state,
        stream_id,
        session_id,
        &user_id,
        StopVoiceVideoStream {
            server_id,
            room_id,
            source: VoiceVideoStreamSource::Camera,
        },
    )
    .await
    .expect("joined user should stop local video stream");
}

#[tokio::test]
async fn stop_video_stream_rejects_stale_session() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let session_id = uuid::Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;

    join_room(
        &state,
        stream_id,
        session_id,
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .expect("join should succeed");

    let error = stop_video_stream(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &user_id,
        StopVoiceVideoStream {
            server_id,
            room_id,
            source: VoiceVideoStreamSource::ScreenShare,
        },
    )
    .await
    .expect_err("stale session should be rejected");

    assert!(matches!(error, VoiceChatApplicationError::Unauthorized(_)));
}
