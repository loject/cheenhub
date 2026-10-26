//! Проверки публикации состояния входящего звука участника голосовой комнаты.

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, RealtimeEnvelope, RealtimeKind, RealtimeModule, SetVoiceOutputState,
    VoiceChatKind, VoiceRoomSnapshot,
};
use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use super::{create_room, join_room, registered_user, state};
use crate::realtime::{EnvelopeSink, WebSocketOutbound};

#[tokio::test]
async fn output_mute_updates_presence_and_fans_out_only_when_changed() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    let joined = join_room(
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
    assert!(!joined.participants[0].output_muted);

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
    let (sender, mut replies) = tokio::sync::mpsc::channel(4);
    let sink = EnvelopeSink::websocket(sender);
    let request_id = Uuid::new_v4();
    let request = RealtimeEnvelope::new(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::SetOutputState),
        Some(request_id),
        SetVoiceOutputState {
            room_id: room_id.clone(),
            output_muted: true,
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
    assert_eq!(reply.request_id, Some(request_id));
    assert_eq!(
        reply.kind,
        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomSnapshot)
    );
    let snapshot: VoiceRoomSnapshot = serde_json::from_value(reply.payload).unwrap();
    assert!(snapshot.participants[0].output_muted);
    assert!(snapshot.participants[0].microphone_enabled);

    let WebSocketOutbound::PreparedEnvelope(event) = observed.try_recv().unwrap() else {
        panic!("expected participant fanout")
    };
    let event: RealtimeEnvelope = serde_json::from_str(&event).unwrap();
    assert_eq!(
        event.kind,
        RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged)
    );
    let broadcast: VoiceRoomSnapshot = serde_json::from_value(event.payload).unwrap();
    assert!(broadcast.participants[0].output_muted);

    crate::features::voice_chat::realtime::handle(
        &state, &user, &user_id, stream_id, session_id, &sink, request,
    )
    .await
    .unwrap();
    assert!(
        observed.try_recv().is_err(),
        "unchanged output choice must not fan out again"
    );
}

#[tokio::test]
async fn output_state_rejects_foreign_stream_user_or_room() {
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
            server_id,
            room_id: room_id.clone(),
            microphone_enabled: true,
        },
    )
    .await
    .unwrap();

    for (candidate_stream, candidate_user, candidate_room) in [
        (Uuid::new_v4(), user_id, room_id.clone()),
        (stream_id, Uuid::new_v4(), room_id.clone()),
        (stream_id, user_id, Uuid::new_v4().to_string()),
    ] {
        let result = crate::features::voice_chat::application::set_output_state(
            &state,
            candidate_stream,
            &candidate_user,
            SetVoiceOutputState {
                room_id: candidate_room,
                output_muted: true,
            },
        )
        .await;
        assert!(
            result.is_err(),
            "only the active owner and room may change output state"
        );
    }
}
