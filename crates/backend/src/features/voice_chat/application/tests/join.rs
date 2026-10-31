//! Проверки ответа на присоединение к серверной голосовой комнате.

use cheenhub_contracts::realtime::JoinVoiceRoom;
use cheenhub_contracts::rest::ServerRoomKind;

use super::{create_room, join_room, registered_user, state};

#[tokio::test]
async fn legacy_evicted_voice_stream_is_not_left_ghost_connected() {
    use crate::features::voice_chat::infrastructure::{VoicePresence, VoicePresenceTargetKind};
    use crate::realtime::{EnvelopeSink, WebSocketOutbound};
    use cheenhub_contracts::realtime::{RealtimeModule, VoiceDeviceKind};
    use chrono::Utc;
    use uuid::Uuid;

    let state = state();
    let (_, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let auth_session_id = Uuid::new_v4();
    let old = VoicePresence {
        microphone_enabled: true,
        output_muted: false,
        realtime_stream_id: stream_id,
        session_id: Uuid::new_v4(),
        auth_session_id,
        device_kind: VoiceDeviceKind::Desktop,
        voice_protocol_version: 4,
        target_kind: VoicePresenceTargetKind::Server,
        server_id: Uuid::new_v4(),
        room_id: Uuid::new_v4(),
        user_id,
        nickname: "Старый клиент".into(),
        avatar_url: None,
        joined_at: Utc::now(),
    };
    let incoming = VoicePresence {
        session_id: Uuid::new_v4(),
        realtime_stream_id: Uuid::new_v4(),
        voice_protocol_version: 5,
        ..old.clone()
    };
    let (outbound, mut notifications) = tokio::sync::mpsc::channel(2);
    state
        .realtime_hub
        .register_test_session_with_id(old.session_id, user_id, auth_session_id)
        .await;
    state
        .realtime_hub
        .register_stream(
            stream_id,
            RealtimeModule::VoiceChat,
            user_id,
            EnvelopeSink::websocket(outbound),
        )
        .await;

    super::super::presence::fanout_join_evictions(&state, std::slice::from_ref(&old), &incoming)
        .await;

    assert!(matches!(
        notifications.try_recv(),
        Ok(WebSocketOutbound::PreparedEnvelope(_))
    ));
    assert_eq!(
        state
            .realtime_hub
            .auth_session_for_transport(&old.session_id, &user_id),
        Some(auth_session_id)
    );
}

#[tokio::test]
async fn legacy_client_conflict_requires_upgrade_without_replacing_existing_presence() {
    use cheenhub_contracts::realtime::VoiceDeviceKind;
    use uuid::Uuid;

    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (first_server, first_room) =
        create_room(&state, &user_id, "first", ServerRoomKind::Voice).await;
    let (second_server, second_room) =
        create_room(&state, &user_id, "second", ServerRoomKind::Voice).await;
    let request = |server_id: String, room_id: String| JoinVoiceRoom {
        server_id,
        room_id,
        microphone_enabled: true,
        voice_protocol_version: 4,
        device_kind: VoiceDeviceKind::Desktop,
        confirm_disconnect_other_devices: false,
    };

    join_room(
        &state,
        Uuid::new_v4(),
        Uuid::new_v4(),
        &user,
        &user_id,
        request(first_server, first_room),
    )
    .await
    .expect("first device joins");
    let result = join_room(
        &state,
        Uuid::new_v4(),
        Uuid::new_v4(),
        &user,
        &user_id,
        request(second_server, second_room),
    )
    .await;

    assert!(matches!(
        result,
        Err(super::super::VoiceChatApplicationError::BadRequest(message))
            if message.contains("Обновите CheenHub")
    ));
    assert_eq!(
        state
            .voice_presence_store
            .active_voice_connection_count()
            .await,
        1
    );
}

#[tokio::test]
async fn legacy_client_cannot_force_device_transition_with_confirmation_flag() {
    use cheenhub_contracts::realtime::VoiceDeviceKind;
    use uuid::Uuid;

    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (first_server, first_room) =
        create_room(&state, &user_id, "original", ServerRoomKind::Voice).await;
    let (second_server, second_room) =
        create_room(&state, &user_id, "destination", ServerRoomKind::Voice).await;
    let request = |server_id: String, room_id: String, confirmed: bool| JoinVoiceRoom {
        server_id,
        room_id,
        microphone_enabled: true,
        voice_protocol_version: 4,
        device_kind: VoiceDeviceKind::Desktop,
        confirm_disconnect_other_devices: confirmed,
    };

    join_room(
        &state,
        Uuid::new_v4(),
        Uuid::new_v4(),
        &user,
        &user_id,
        request(first_server, first_room, false),
    )
    .await
    .expect("first device joins");
    let result = join_room(
        &state,
        Uuid::new_v4(),
        Uuid::new_v4(),
        &user,
        &user_id,
        request(second_server, second_room, true),
    )
    .await;

    assert!(matches!(
        result,
        Err(super::super::VoiceChatApplicationError::BadRequest(message))
            if message.contains("Обновите CheenHub")
    ));
    assert_eq!(
        state
            .voice_presence_store
            .active_voice_connection_count()
            .await,
        1
    );
}

#[tokio::test]
async fn joining_with_microphone_disabled_includes_muted_state_in_snapshot() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    let request = serde_json::from_value(serde_json::json!({
        "server_id": server_id,
        "room_id": room_id,
        "microphone_enabled": false,
        "voice_protocol_version": 4,
    }))
    .expect("valid join request");

    let snapshot = join_room(
        &state,
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        request,
    )
    .await
    .expect("join succeeds");

    let participant = serde_json::to_value(&snapshot.participants[0]).unwrap();
    assert_eq!(participant["microphone_enabled"], false);
}

#[tokio::test]
async fn legacy_join_without_protocol_version_is_still_allowed() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (server_id, room_id) = create_room(&state, &user_id, "legacy", ServerRoomKind::Voice).await;
    let request: JoinVoiceRoom = serde_json::from_value(serde_json::json!({
        "server_id": server_id,
        "room_id": room_id
    }))
    .unwrap();

    let snapshot = join_room(
        &state,
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        request,
    )
    .await
    .expect("клиент без нового поля версии должен входить в голосовую комнату");

    assert_eq!(snapshot.participants.len(), 1);
    assert_eq!(snapshot.self_session_id, None);
}

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
            microphone_enabled: true,
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            voice_protocol_version: 4,
            device_kind: cheenhub_contracts::realtime::VoiceDeviceKind::Desktop,
            confirm_disconnect_other_devices: false,
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
            microphone_enabled: true,
            output_muted: false,
            realtime_stream_id: stream_id,
            session_id: uuid::Uuid::new_v4(),
            target_kind: target.kind,
            server_id: target.server_id,
            room_id: target.room_id,
            user_id,
            nickname: "Участник".into(),
            avatar_url: None,
            joined_at: chrono::Utc::now(),
            auth_session_id: uuid::Uuid::new_v4(),
            device_kind: cheenhub_contracts::realtime::VoiceDeviceKind::Desktop,
            voice_protocol_version: 4,
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
        microphone_enabled: true,
        output_muted: false,
        realtime_stream_id: old_stream,
        session_id: uuid::Uuid::new_v4(),
        auth_session_id: uuid::Uuid::new_v4(),
        device_kind: cheenhub_contracts::realtime::VoiceDeviceKind::Desktop,
        voice_protocol_version: 4,
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
            microphone_enabled: true,
            output_muted: false,
            realtime_stream_id: stream_id,
            session_id: uuid::Uuid::new_v4(),
            target_kind: target.kind,
            server_id: target.server_id,
            room_id: target.room_id,
            user_id,
            nickname: "Участник".into(),
            avatar_url: None,
            joined_at: chrono::Utc::now(),
            auth_session_id: uuid::Uuid::new_v4(),
            device_kind: cheenhub_contracts::realtime::VoiceDeviceKind::Desktop,
            voice_protocol_version: 4,
        })
        .await;

    let result =
        super::super::presence::finish_server_join(&state, stream_id, target, replaced, Ok(false))
            .await;

    assert!(matches!(
        result,
        Err(super::super::VoiceChatApplicationError::NotFound(_))
    ));
    let crate::realtime::WebSocketOutbound::PreparedEnvelope(json) =
        notifications.try_recv().unwrap()
    else {
        panic!("ожидался снимок покинутой комнаты");
    };
    let envelope: cheenhub_contracts::realtime::RealtimeEnvelope =
        serde_json::from_str(json.as_str()).unwrap();
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
