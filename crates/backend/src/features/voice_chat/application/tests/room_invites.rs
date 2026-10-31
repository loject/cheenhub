//! Проверки доступа и сценариев приглашения в серверную голосовую комнату.

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, ListVoiceRoomInviteCandidates, ListVoiceRoomInvites, RespondVoiceRoomInvite,
    SendVoiceRoomInvite, VoiceDeviceKind, VoiceRoomInviteResponse, VoiceRoomInviteStatus,
};
use cheenhub_contracts::rest::{RegisterRequest, ServerRoomKind};
use uuid::Uuid;

use super::{create_room, join_room, registered_user, state};
use crate::features::auth::application as auth_application;
use crate::features::voice_chat::application::{
    VoiceChatApplicationError, list_voice_room_invite_candidates, list_voice_room_invites,
    respond_voice_room_invite, send_voice_room_invite,
};
use crate::state::AppState;

async fn another_user(state: &AppState) -> (cheenhub_contracts::rest::AuthUser, Uuid) {
    let registered = auth_application::register(
        state,
        RegisterRequest {
            nickname: "invite_callee".to_owned(),
            email: "invite-callee@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .unwrap();
    let id = Uuid::parse_str(&registered.user.id).unwrap();
    (registered.user, id)
}

async fn caller_in_room(
    state: &AppState,
    caller: &cheenhub_contracts::rest::AuthUser,
    caller_id: &Uuid,
    server_id: &str,
    room_id: &str,
) -> Uuid {
    let stream_id = Uuid::new_v4();
    join_room(
        state,
        stream_id,
        Uuid::new_v4(),
        caller,
        caller_id,
        JoinVoiceRoom {
            microphone_enabled: true,
            server_id: server_id.to_owned(),
            room_id: room_id.to_owned(),
            voice_protocol_version: 5,
            device_kind: VoiceDeviceKind::Desktop,
            confirm_disconnect_other_devices: false,
        },
    )
    .await
    .unwrap();
    stream_id
}

#[tokio::test]
async fn voice_room_invite_requires_active_sender_and_target_membership() {
    let state = state();
    let (caller, caller_id) = registered_user(&state).await;
    let (_, callee_id) = another_user(&state).await;
    let (server_id, room_id) =
        create_room(&state, &caller_id, "voice", ServerRoomKind::Voice).await;
    let request = SendVoiceRoomInvite {
        server_id: server_id.clone(),
        room_id: room_id.clone(),
        recipient_user_id: callee_id.to_string(),
    };
    let stream = Uuid::new_v4();
    assert!(matches!(
        send_voice_room_invite(&state, stream, &caller, &caller_id, request.clone()).await,
        Err(VoiceChatApplicationError::Unauthorized(_))
    ));
    let stream = caller_in_room(&state, &caller, &caller_id, &server_id, &room_id).await;
    assert!(matches!(
        send_voice_room_invite(&state, stream, &caller, &caller_id, request.clone()).await,
        Err(VoiceChatApplicationError::Unauthorized(_))
    ));
    state
        .server_store
        .insert_server_member(&Uuid::parse_str(&server_id).unwrap(), &callee_id)
        .await
        .unwrap();
    let event = send_voice_room_invite(&state, stream, &caller, &caller_id, request)
        .await
        .unwrap();
    assert_eq!(event.invite.status, VoiceRoomInviteStatus::Pending);
}

#[tokio::test]
async fn voice_room_invite_accepts_once_and_restores_snapshots() {
    let state = state();
    let (caller, caller_id) = registered_user(&state).await;
    let (_, callee_id) = another_user(&state).await;
    let (server_id, room_id) =
        create_room(&state, &caller_id, "voice", ServerRoomKind::Voice).await;
    state
        .server_store
        .insert_server_member(&Uuid::parse_str(&server_id).unwrap(), &callee_id)
        .await
        .unwrap();
    let stream = caller_in_room(&state, &caller, &caller_id, &server_id, &room_id).await;
    let sent = send_voice_room_invite(
        &state,
        stream,
        &caller,
        &caller_id,
        SendVoiceRoomInvite {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            recipient_user_id: callee_id.to_string(),
        },
    )
    .await
    .unwrap();
    let incoming = list_voice_room_invites(&state, &callee_id, ListVoiceRoomInvites)
        .await
        .unwrap();
    assert_eq!(incoming.invites.len(), 1);
    assert_eq!(incoming.invites[0].invite_id, sent.invite.invite_id);
    assert!(matches!(
        respond_voice_room_invite(
            &state,
            &caller_id,
            RespondVoiceRoomInvite {
                invite_id: sent.invite.invite_id.clone(),
                response: VoiceRoomInviteResponse::Decline,
            }
        )
        .await,
        Err(VoiceChatApplicationError::Unauthorized(_))
    ));
    let accepted = respond_voice_room_invite(
        &state,
        &callee_id,
        RespondVoiceRoomInvite {
            invite_id: sent.invite.invite_id.clone(),
            response: VoiceRoomInviteResponse::Accept,
        },
    )
    .await
    .unwrap();
    assert_eq!(accepted.invite.status, VoiceRoomInviteStatus::Accepted);
    assert_eq!(
        list_voice_room_invites(&state, &caller_id, ListVoiceRoomInvites)
            .await
            .unwrap()
            .invites[0]
            .status,
        VoiceRoomInviteStatus::Accepted
    );
    assert!(
        respond_voice_room_invite(
            &state,
            &callee_id,
            RespondVoiceRoomInvite {
                invite_id: sent.invite.invite_id,
                response: VoiceRoomInviteResponse::Accept,
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn voice_room_invite_candidates_exclude_current_voice_participants() {
    let state = state();
    let (caller, caller_id) = registered_user(&state).await;
    let (callee, callee_id) = another_user(&state).await;
    let (server_id, room_id) =
        create_room(&state, &caller_id, "voice", ServerRoomKind::Voice).await;
    state
        .server_store
        .insert_server_member(&Uuid::parse_str(&server_id).unwrap(), &callee_id)
        .await
        .unwrap();
    let stream = caller_in_room(&state, &caller, &caller_id, &server_id, &room_id).await;
    let request = ListVoiceRoomInviteCandidates {
        server_id: server_id.clone(),
        room_id: room_id.clone(),
    };
    let candidates = list_voice_room_invite_candidates(&state, stream, &caller_id, request.clone())
        .await
        .unwrap();
    assert_eq!(candidates.candidates.len(), 1);
    assert_eq!(candidates.candidates[0].user_id, callee_id.to_string());
    caller_in_room(&state, &callee, &callee_id, &server_id, &room_id).await;
    assert!(
        list_voice_room_invite_candidates(&state, stream, &caller_id, request)
            .await
            .unwrap()
            .candidates
            .is_empty()
    );
}
