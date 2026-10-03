//! Проверки ответа на присоединение к серверной голосовой комнате.

use cheenhub_contracts::realtime::JoinVoiceRoom;
use cheenhub_contracts::rest::ServerRoomKind;

use super::{create_room, join_room, registered_user, state};

#[tokio::test]
async fn joining_server_voice_room_returns_server_audio_bitrate() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    let server_uuid = uuid::Uuid::parse_str(&server_id).expect("server id should be uuid");
    state
        .server_store
        .update_server_audio_bitrate(&server_uuid, &user_id, 48_000)
        .await
        .expect("bitrate should update");

    let snapshot = join_room(
        &state,
        uuid::Uuid::new_v4(),
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

    assert_eq!(snapshot.audio_bitrate_bps, Some(48_000));
    assert_eq!(snapshot.server_id, server_id);
    assert_eq!(snapshot.room_id, room_id);
}

#[tokio::test]
async fn failed_post_join_lookup_removes_rejected_presence() {
    use crate::features::voice_chat::infrastructure::{
        VoicePresence, VoicePresenceTarget, VoicePresenceTargetKind,
    };
    let state = state();
    let (_, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let target = VoicePresenceTarget {
        kind: VoicePresenceTargetKind::Server,
        server_id: uuid::Uuid::new_v4(),
        room_id: uuid::Uuid::new_v4(),
    };
    state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id: stream_id,
            session_id: uuid::Uuid::new_v4(),
            target_kind: target.kind,
            server_id: target.server_id,
            room_id: target.room_id,
            user_id,
            nickname: "Участник".into(),
            avatar_url: None,
            joined_at: chrono::Utc::now(),
        })
        .await;

    let result = super::super::presence::finish_server_join(
        &state,
        stream_id,
        target,
        Vec::new(),
        Err(anyhow::anyhow!("database unavailable")),
    )
    .await;

    assert!(matches!(
        result,
        Err(super::super::VoiceChatApplicationError::Internal(_))
    ));
    assert_eq!(
        state
            .voice_presence_store
            .active_voice_connection_count()
            .await,
        0
    );
}

#[tokio::test]
async fn deleted_join_target_still_notifies_replaced_room() {
    use crate::features::voice_chat::infrastructure::{
        VoicePresence, VoicePresenceTarget, VoicePresenceTargetKind,
    };
    let state = state();
    let (_, user_id) = registered_user(&state).await;
    let (old_server, old_room) = create_room(&state, &user_id, "old", ServerRoomKind::Voice).await;
    let old_stream = uuid::Uuid::new_v4();
    let (outbound, mut notifications) = tokio::sync::mpsc::channel(2);
    state
        .realtime_hub
        .register_stream(
            old_stream,
            cheenhub_contracts::realtime::RealtimeModule::VoiceChat,
            user_id,
            crate::realtime::EnvelopeSink::websocket(outbound),
        )
        .await;
    let old = VoicePresence {
        realtime_stream_id: old_stream,
        session_id: uuid::Uuid::new_v4(),
        target_kind: VoicePresenceTargetKind::Server,
        server_id: uuid::Uuid::parse_str(&old_server).unwrap(),
        room_id: uuid::Uuid::parse_str(&old_room).unwrap(),
        user_id,
        nickname: "Участник".into(),
        avatar_url: None,
        joined_at: chrono::Utc::now(),
    };
    state.voice_presence_store.join(old).await;
    let stream_id = uuid::Uuid::new_v4();
    let target = VoicePresenceTarget {
        kind: VoicePresenceTargetKind::Server,
        server_id: uuid::Uuid::new_v4(),
        room_id: uuid::Uuid::new_v4(),
    };
    let replaced = state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id: stream_id,
            session_id: uuid::Uuid::new_v4(),
            target_kind: target.kind,
            server_id: target.server_id,
            room_id: target.room_id,
            user_id,
            nickname: "Участник".into(),
            avatar_url: None,
            joined_at: chrono::Utc::now(),
        })
        .await;

    let result =
        super::super::presence::finish_server_join(&state, stream_id, target, replaced, Ok(false))
            .await;

    assert!(matches!(
        result,
        Err(super::super::VoiceChatApplicationError::NotFound(_))
    ));
    let crate::realtime::WebSocketOutbound::Envelope(envelope) = notifications.try_recv().unwrap()
    else {
        panic!("ожидался снимок покинутой комнаты");
    };
    let snapshot: cheenhub_contracts::realtime::VoiceRoomSnapshot =
        serde_json::from_value(envelope.payload).unwrap();
    assert_eq!(snapshot.room_id, old_room);
    assert!(snapshot.participants.is_empty());
    assert_eq!(
        state
            .voice_presence_store
            .active_voice_connection_count()
            .await,
        0
    );
}
