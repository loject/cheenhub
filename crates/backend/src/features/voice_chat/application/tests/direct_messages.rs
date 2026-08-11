//! Тесты голосовых комнат личных диалогов.

use cheenhub_contracts::realtime::{
    DirectCallResponse, DirectCallState, JoinDirectMessageVoiceRoom, LeaveDirectMessageVoiceRoom,
    ListDirectCalls, ListDirectMessageVoiceRooms, RespondDirectCall, StartDirectCall,
};
use cheenhub_contracts::rest::RegisterRequest;
use chrono::{Duration, Utc};

use super::state;
use crate::features::auth::application as auth_application;
use crate::features::social::{accept_friend_request, open_dm_conversation, send_friend_request};
use crate::features::voice_chat::application::direct_calls::{
    expire_direct_calls_at, list_direct_calls, respond_direct_call, start_direct_call,
    start_direct_call_without_expiry_task,
};
use crate::features::voice_chat::application::{
    VoiceChatApplicationError, disconnect_realtime_stream, join_direct_message_room,
    leave_direct_message_room, list_direct_message_voice_rooms,
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
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed");
    let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should be uuid");

    (auth, user_id)
}

async fn conversation_between(
    state: &AppState,
    requester: &cheenhub_contracts::rest::AuthResponse,
    recipient: &cheenhub_contracts::rest::AuthResponse,
) -> String {
    let request = send_friend_request(
        state,
        &requester.access_token,
        cheenhub_contracts::rest::SendFriendRequestRequest {
            recipient_user_id: recipient.user.id.clone(),
        },
    )
    .await
    .expect("friend request should send");
    accept_friend_request(state, &recipient.access_token, request.request.id)
        .await
        .expect("friend request should accept");
    open_dm_conversation(
        state,
        &requester.access_token,
        cheenhub_contracts::rest::OpenDmConversationRequest {
            friend_user_id: recipient.user.id.clone(),
        },
    )
    .await
    .expect("conversation should open")
    .conversation
    .id
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
    let conversation_id = conversation_between(state, &alice_auth, &bob_auth).await;

    (
        alice_auth.user,
        alice_id,
        bob_auth.user,
        bob_id,
        conversation_id,
    )
}

#[tokio::test]
async fn callee_can_accept_direct_call_and_both_users_can_recover_it() {
    let state = state();
    let (alice, alice_id, _bob, bob_id, conversation_id) =
        direct_message_conversation(&state).await;

    let started = start_direct_call(
        &state,
        &alice,
        &alice_id,
        StartDirectCall {
            conversation_id: conversation_id.clone(),
        },
    )
    .await
    .expect("direct call should start");
    assert_eq!(started.call.state, DirectCallState::Ringing);
    assert_eq!(started.call.caller_nickname, "voice_alice");
    assert_eq!(started.call.callee_nickname, "voice_bob");

    let accepted = respond_direct_call(
        &state,
        &bob_id,
        RespondDirectCall {
            call_id: started.call.call_id.clone(),
            response: DirectCallResponse::Accept,
        },
    )
    .await
    .expect("callee should accept");
    assert_eq!(accepted.call.state, DirectCallState::Active);
    assert!(accepted.call.answered_at.is_some());

    let alice_calls = list_direct_calls(&state, &alice_id, ListDirectCalls)
        .await
        .expect("caller should recover active call");
    let bob_calls = list_direct_calls(&state, &bob_id, ListDirectCalls)
        .await
        .expect("callee should recover active call");
    assert_eq!(alice_calls.calls, bob_calls.calls);
    assert_eq!(alice_calls.calls.len(), 1);
    assert_eq!(alice_calls.calls[0].conversation_id, conversation_id);
}

#[tokio::test]
async fn disconnecting_direct_message_presence_ends_active_call() {
    let state = state();
    let (alice, alice_id, _bob, bob_id, conversation_id) =
        direct_message_conversation(&state).await;
    let started = start_direct_call(
        &state,
        &alice,
        &alice_id,
        StartDirectCall {
            conversation_id: conversation_id.clone(),
        },
    )
    .await
    .expect("direct call should start");
    respond_direct_call(
        &state,
        &bob_id,
        RespondDirectCall {
            call_id: started.call.call_id,
            response: DirectCallResponse::Accept,
        },
    )
    .await
    .expect("callee should accept");
    let stream_id = uuid::Uuid::new_v4();
    join_direct_message_room(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &alice,
        &alice_id,
        JoinDirectMessageVoiceRoom { conversation_id },
    )
    .await
    .expect("caller should join media presence");

    disconnect_realtime_stream(&state, stream_id).await;

    assert!(
        list_direct_calls(&state, &bob_id, ListDirectCalls)
            .await
            .expect("calls should list")
            .calls
            .is_empty()
    );
}

#[tokio::test]
async fn callee_can_decline_direct_call() {
    let state = state();
    let (alice, alice_id, _bob, bob_id, conversation_id) =
        direct_message_conversation(&state).await;
    let started = start_direct_call(
        &state,
        &alice,
        &alice_id,
        StartDirectCall { conversation_id },
    )
    .await
    .expect("direct call should start");

    let declined = respond_direct_call(
        &state,
        &bob_id,
        RespondDirectCall {
            call_id: started.call.call_id,
            response: DirectCallResponse::Decline,
        },
    )
    .await
    .expect("callee should decline");

    assert_eq!(declined.call.state, DirectCallState::Ended);
    assert_eq!(
        declined.call.end_reason,
        Some(cheenhub_contracts::realtime::DirectCallEndReason::Declined)
    );
    assert!(
        list_direct_calls(&state, &alice_id, ListDirectCalls)
            .await
            .expect("calls should list")
            .calls
            .is_empty()
    );
}

#[tokio::test]
async fn unrelated_user_cannot_respond_to_direct_call() {
    let state = state();
    let (alice, alice_id, _bob, _bob_id, conversation_id) =
        direct_message_conversation(&state).await;
    let (_mallory, mallory_id) =
        registered_user_with_email(&state, "voice_mallory", "voice-mallory@example.com").await;
    let started = start_direct_call(
        &state,
        &alice,
        &alice_id,
        StartDirectCall { conversation_id },
    )
    .await
    .expect("direct call should start");

    let error = respond_direct_call(
        &state,
        &mallory_id,
        RespondDirectCall {
            call_id: started.call.call_id,
            response: DirectCallResponse::Accept,
        },
    )
    .await
    .expect_err("unrelated user should be rejected");

    assert!(matches!(error, VoiceChatApplicationError::Unauthorized(_)));
}

#[tokio::test]
async fn caller_cannot_start_duplicate_direct_call() {
    let state = state();
    let (alice, alice_id, _bob, _bob_id, conversation_id) =
        direct_message_conversation(&state).await;
    start_direct_call(
        &state,
        &alice,
        &alice_id,
        StartDirectCall {
            conversation_id: conversation_id.clone(),
        },
    )
    .await
    .expect("first direct call should start");

    let error = start_direct_call(
        &state,
        &alice,
        &alice_id,
        StartDirectCall { conversation_id },
    )
    .await
    .expect_err("duplicate direct call should be rejected");

    assert!(matches!(error, VoiceChatApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn busy_callee_is_hidden_from_second_caller() {
    let state = state();
    let (alice, alice_id) =
        registered_user_with_email(&state, "privacy_alice", "privacy-alice@example.com").await;
    let (bob, bob_id) =
        registered_user_with_email(&state, "privacy_bob", "privacy-bob@example.com").await;
    let (carol, carol_id) =
        registered_user_with_email(&state, "privacy_carol", "privacy-carol@example.com").await;
    let alice_bob_conversation = conversation_between(&state, &alice, &bob).await;
    let carol_bob_conversation = conversation_between(&state, &carol, &bob).await;

    start_direct_call(
        &state,
        &alice.user,
        &alice_id,
        StartDirectCall {
            conversation_id: alice_bob_conversation,
        },
    )
    .await
    .expect("first call should start");
    let hidden = start_direct_call(
        &state,
        &carol.user,
        &carol_id,
        StartDirectCall {
            conversation_id: carol_bob_conversation,
        },
    )
    .await
    .expect("second caller should see a ringing call");

    assert_eq!(hidden.call.state, DirectCallState::Ringing);
    assert_eq!(
        list_direct_calls(&state, &carol_id, ListDirectCalls)
            .await
            .expect("second caller should recover ringing call")
            .calls,
        vec![hidden.call.clone()]
    );
    let bob_calls = list_direct_calls(&state, &bob_id, ListDirectCalls)
        .await
        .expect("busy callee should recover only the original call");
    assert_eq!(bob_calls.calls.len(), 1);
    assert_ne!(bob_calls.calls[0].call_id, hidden.call.call_id);
}

#[tokio::test]
async fn unanswered_direct_call_expires() {
    let state = state();
    let (alice, alice_id, _bob, bob_id, conversation_id) =
        direct_message_conversation(&state).await;
    let now = Utc::now();
    let started = start_direct_call_without_expiry_task(
        &state,
        &alice,
        &alice_id,
        StartDirectCall { conversation_id },
        now,
        Duration::seconds(1),
    )
    .await
    .expect("direct call should start");

    expire_direct_calls_at(&state, now + Duration::seconds(2)).await;

    assert_eq!(started.call.state, DirectCallState::Ringing);
    assert!(
        list_direct_calls(&state, &bob_id, ListDirectCalls)
            .await
            .expect("calls should list")
            .calls
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
