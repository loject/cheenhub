//! Регрессии выдачи независимых разрешений микрофона нескольким устройствам.

use cheenhub_contracts::realtime::{
    BindMicrophoneUplink, IssueMicrophoneUplinkGrant, JoinVoiceRoom, VoiceDeviceKind,
};
use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use super::{
    bind_microphone_uplink, create_room, issue_microphone_uplink_grant, join_room, registered_user,
    state,
};

#[tokio::test]
async fn two_auth_devices_can_bind_independent_microphone_workers_in_one_room() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    let mut sessions = Vec::new();
    for _ in 0..2 {
        let session_id = Uuid::new_v4();
        join_room(
            &state,
            Uuid::new_v4(),
            session_id,
            &user,
            &user_id,
            JoinVoiceRoom {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                microphone_enabled: true,
                voice_protocol_version: 4,
                device_kind: VoiceDeviceKind::Desktop,
                confirm_disconnect_other_devices: false,
            },
        )
        .await
        .expect("разные auth-сессии могут находиться в одной комнате");
        sessions.push(session_id);
    }

    let mut grants = Vec::new();
    for session_id in &sessions {
        let issued = issue_microphone_uplink_grant(
            &state,
            *session_id,
            &user_id,
            IssueMicrophoneUplinkGrant {
                room_id: room_id.clone(),
            },
        )
        .await
        .expect("каждая сессия должна получить собственное разрешение");
        grants.push(issued.grant);
    }

    for grant in grants {
        let bound = bind_microphone_uplink(
            &state,
            Uuid::new_v4(),
            &user_id,
            BindMicrophoneUplink { grant },
        )
        .await
        .expect("разрешение одного устройства не должно отменять другое");
        assert_eq!(bound.room_id, room_id);
    }
}
