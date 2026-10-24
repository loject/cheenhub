//! Проверки смены микрофона через realtime и снимков голосового присутствия.

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, RealtimeEnvelope, RealtimeKind, RealtimeModule, SetVoiceMicrophoneState,
    VoiceChatKind, VoiceRoomSnapshot,
};
use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use super::{create_room, join_room, registered_user, state};
use crate::realtime::{EnvelopeSink, WebSocketOutbound};

#[tokio::test]
async fn queued_participant_fanouts_preserve_concurrent_microphone_changes() {
    use crate::features::voice_chat::application::{
        fanout::server_voice_target, set_microphone_state,
    };
    use futures_util::FutureExt;
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let peer_id = Uuid::new_v4();
    let peer_stream = Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    let target = server_voice_target(
        Uuid::parse_str(&server_id).unwrap(),
        Uuid::parse_str(&room_id).unwrap(),
    );
    state
        .server_store
        .insert_server_member(&target.server_id, &peer_id)
        .await
        .unwrap();
    for (stream, id) in [(stream_id, user_id), (peer_stream, peer_id)] {
        let mut participant = user.clone();
        participant.id = id.to_string();
        join_room(
            &state,
            stream,
            Uuid::new_v4(),
            &participant,
            &id,
            JoinVoiceRoom {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                microphone_enabled: true,
            },
        )
        .await
        .unwrap();
    }
    let (observer, mut observed) = tokio::sync::mpsc::channel(4);
    state
        .realtime_hub
        .register_stream(
            Uuid::new_v4(),
            RealtimeModule::VoiceChat,
            user_id,
            EnvelopeSink::websocket(observer),
        )
        .await;
    let publication = state
        .voice_presence_store
        .snapshot_publication_lock(target)
        .await;
    let previous_publication = publication.lock().await;
    let request = SetVoiceMicrophoneState {
        room_id,
        microphone_enabled: false,
    };
    let mut first = Box::pin(set_microphone_state(
        &state,
        stream_id,
        &user_id,
        request.clone(),
    ));
    let mut second = Box::pin(set_microphone_state(&state, peer_stream, &peer_id, request));

    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    drop(previous_publication);
    let (first, second) = tokio::join!(first, second);
    first.unwrap();
    second.unwrap();

    for _ in 0..2 {
        let WebSocketOutbound::PreparedEnvelope(event) = observed.try_recv().unwrap() else {
            panic!("expected fanout")
        };
        let event: RealtimeEnvelope = serde_json::from_str(&event).unwrap();
        let broadcast: VoiceRoomSnapshot = serde_json::from_value(event.payload).unwrap();
        assert_eq!(broadcast.participants.len(), 2);
        assert!(
            broadcast
                .participants
                .iter()
                .all(|participant| !participant.microphone_enabled)
        );
    }
}

#[tokio::test]
async fn participant_fanout_does_not_publish_a_previously_captured_microphone_choice() {
    use crate::features::voice_chat::application::fanout::{fanout_snapshot, server_voice_target};
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    join_room(
        &state,
        stream_id,
        Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            microphone_enabled: true,
        },
    )
    .await
    .unwrap();
    let target = server_voice_target(
        Uuid::parse_str(&server_id).unwrap(),
        Uuid::parse_str(&room_id).unwrap(),
    );
    let (observer, mut observed) = tokio::sync::mpsc::channel(4);
    state
        .realtime_hub
        .register_stream(
            Uuid::new_v4(),
            RealtimeModule::VoiceChat,
            user_id,
            EnvelopeSink::websocket(observer),
        )
        .await;
    state
        .voice_presence_store
        .set_microphone_enabled(&stream_id, &user_id, &target.room_id, false)
        .await
        .unwrap();

    fanout_snapshot(&state, target).await;

    let WebSocketOutbound::PreparedEnvelope(event) = observed.try_recv().unwrap() else {
        panic!("expected fanout")
    };
    let event: RealtimeEnvelope = serde_json::from_str(&event).unwrap();
    let broadcast: VoiceRoomSnapshot = serde_json::from_value(event.payload).unwrap();
    assert!(!broadcast.participants[0].microphone_enabled);
}

#[tokio::test]
async fn microphone_change_rejects_foreign_stream_user_room_and_disconnected_presence() {
    use crate::features::voice_chat::application::{
        VoiceChatApplicationError, set_microphone_state,
    };
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    join_room(
        &state,
        stream_id,
        Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            microphone_enabled: true,
        },
    )
    .await
    .unwrap();

    for (stream, user, room) in [
        (Uuid::new_v4(), user_id, room_id.clone()),
        (stream_id, Uuid::new_v4(), room_id.clone()),
        (stream_id, user_id, Uuid::new_v4().to_string()),
    ] {
        let result = set_microphone_state(
            &state,
            stream,
            &user,
            SetVoiceMicrophoneState {
                room_id: room,
                microphone_enabled: false,
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(VoiceChatApplicationError::Unauthorized(_))
        ));
    }
    let listed = super::list_server_voice_rooms(
        &state,
        &user_id,
        cheenhub_contracts::realtime::ListServerVoiceRooms {
            server_id: server_id.clone(),
        },
    )
    .await
    .unwrap();
    assert!(listed.rooms[0].participants[0].microphone_enabled);

    super::leave_room(
        &state,
        stream_id,
        &user_id,
        cheenhub_contracts::realtime::LeaveVoiceRoom {
            server_id,
            room_id: room_id.clone(),
        },
    )
    .await
    .unwrap();
    let result = set_microphone_state(
        &state,
        stream_id,
        &user_id,
        SetVoiceMicrophoneState {
            room_id,
            microphone_enabled: false,
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(VoiceChatApplicationError::Unauthorized(_))
    ));
}

#[tokio::test]
async fn replaced_stream_cannot_unmute_new_presence_for_same_user() {
    use crate::features::voice_chat::application::{
        VoiceChatApplicationError, set_microphone_state,
    };
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    for (stream, enabled) in [(stream_id, true), (Uuid::new_v4(), false)] {
        join_room(
            &state,
            stream,
            Uuid::new_v4(),
            &user,
            &user_id,
            JoinVoiceRoom {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                microphone_enabled: enabled,
            },
        )
        .await
        .unwrap();
    }

    let result = set_microphone_state(
        &state,
        stream_id,
        &user_id,
        SetVoiceMicrophoneState {
            room_id,
            microphone_enabled: true,
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(VoiceChatApplicationError::Unauthorized(_))
    ));
    let listed = super::list_server_voice_rooms(
        &state,
        &user_id,
        cheenhub_contracts::realtime::ListServerVoiceRooms { server_id },
    )
    .await
    .unwrap();
    assert!(!listed.rooms[0].participants[0].microphone_enabled);
}

#[tokio::test]
async fn microphone_request_updates_presence_and_fans_out_snapshot() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
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
            microphone_enabled: true,
        },
    )
    .await
    .unwrap();
    let (observer, mut observed) = tokio::sync::mpsc::channel(4);
    state
        .realtime_hub
        .register_stream(
            Uuid::new_v4(),
            RealtimeModule::VoiceChat,
            user_id,
            EnvelopeSink::websocket(observer),
        )
        .await;
    let (outsider, mut unrelated) = tokio::sync::mpsc::channel(4);
    state
        .realtime_hub
        .register_stream(
            Uuid::new_v4(),
            RealtimeModule::VoiceChat,
            Uuid::new_v4(),
            EnvelopeSink::websocket(outsider),
        )
        .await;
    let (sender, mut replies) = tokio::sync::mpsc::channel(4);
    let sink = EnvelopeSink::websocket(sender);
    let request_id = Uuid::new_v4();
    let request = RealtimeEnvelope::new(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::SetMicrophoneState),
        Some(request_id),
        SetVoiceMicrophoneState {
            room_id: room_id.clone(),
            microphone_enabled: false,
        },
    )
    .unwrap();

    crate::features::voice_chat::realtime::handle(
        &state,
        &user,
        &user_id,
        stream_id,
        session_id,
        &sink,
        request.clone(),
    )
    .await
    .unwrap();

    let WebSocketOutbound::Envelope(reply) = replies.try_recv().unwrap() else {
        panic!("expected response")
    };
    assert_eq!(
        reply.kind,
        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomSnapshot)
    );
    assert_eq!(reply.request_id, Some(request_id));
    let snapshot: VoiceRoomSnapshot = serde_json::from_value(reply.payload).unwrap();
    assert!(!snapshot.participants[0].microphone_enabled);
    let WebSocketOutbound::PreparedEnvelope(event) = observed.try_recv().unwrap() else {
        panic!("expected fanout")
    };
    let event: RealtimeEnvelope = serde_json::from_str(&event).unwrap();
    assert_eq!(
        event.kind,
        RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged)
    );
    let broadcast: VoiceRoomSnapshot = serde_json::from_value(event.payload).unwrap();
    assert!(!broadcast.participants[0].microphone_enabled);
    assert!(unrelated.try_recv().is_err());

    crate::features::voice_chat::realtime::handle(
        &state, &user, &user_id, stream_id, session_id, &sink, request,
    )
    .await
    .unwrap();

    assert!(
        observed.try_recv().is_err(),
        "unchanged choice must not fan out again"
    );
    let listed = super::list_server_voice_rooms(
        &state,
        &user_id,
        cheenhub_contracts::realtime::ListServerVoiceRooms { server_id },
    )
    .await
    .unwrap();
    assert!(!listed.rooms[0].participants[0].microphone_enabled);
}
