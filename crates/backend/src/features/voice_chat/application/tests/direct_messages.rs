//! Тесты голосовых комнат личных диалогов.

use cheenhub_contracts::realtime::{
    JoinDirectMessageVoiceRoom, LeaveDirectMessageVoiceRoom, ListDirectMessageVoiceRooms,
};
use cheenhub_contracts::rest::RegisterRequest;

use super::state;
use crate::features::auth::application as auth_application;
use crate::features::social::{accept_friend_request, open_dm_conversation, send_friend_request};
use crate::features::voice_chat::application::{
    join_direct_message_room, leave_direct_message_room, list_direct_message_voice_rooms,
};
use crate::state::AppState;

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
