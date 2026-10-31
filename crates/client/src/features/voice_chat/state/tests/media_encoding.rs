//! Регрессии выбора формата медиапакетов по версии участников и подтверждению сессии.

use super::*;
use uuid::Uuid;

#[test]
fn media_waits_for_join_ack_and_keeps_v2_with_any_peer_version() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server".into(), "room".into(), "Общая".into());
        let (handle, _) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });
        handle.apply_snapshot(snapshot(
            &target,
            vec![participant(CURRENT_USER_ID), participant(PEER_USER_ID)],
        ));
        assert!(matches!(
            handle.state(),
            VoiceConnectionState::Connecting { .. }
        ));
        assert_eq!(
            handle.outbound_session_id_for(&target.server_id, &target.room_id),
            None
        );

        let session_id = Uuid::new_v4();
        handle.complete_join(
            &target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![participant(CURRENT_USER_ID)]),
                self_session_id: session_id.to_string(),
                audio_bitrate_bps: 48_000,
            },
        );
        assert_eq!(
            handle.outbound_session_id_for(&target.server_id, &target.room_id),
            Some(session_id)
        );
        assert_eq!(
            handle.outbound_session_id_for("another-server", &target.room_id),
            None
        );

        let mut own_participant = participant(CURRENT_USER_ID);
        own_participant.session_id = session_id.to_string();
        let mut older_peer = participant(PEER_USER_ID);
        older_peer.voice_protocol_version = 4;
        handle.apply_snapshot(snapshot(&target, vec![own_participant.clone(), older_peer]));
        assert_eq!(
            handle.outbound_session_id_for(&target.server_id, &target.room_id),
            Some(session_id)
        );
        handle.apply_snapshot(snapshot(&target, vec![own_participant]));
        assert_eq!(
            handle.outbound_session_id_for(&target.server_id, &target.room_id),
            Some(session_id)
        );

        handle.apply_snapshot(snapshot(&target, vec![participant(PEER_USER_ID)]));
        assert_eq!(
            handle.outbound_session_id_for(&target.server_id, &target.room_id),
            None
        );
    });
}

#[test]
fn invalid_join_session_does_not_authorize_media() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server".into(), "room".into(), "Общая".into());
        let (handle, _) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });
        handle.complete_join(
            &target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![participant(CURRENT_USER_ID)]),
                self_session_id: Uuid::nil().to_string(),
                audio_bitrate_bps: 48_000,
            },
        );
        assert!(matches!(handle.state(), VoiceConnectionState::Error { .. }));
        assert_eq!(
            handle.outbound_session_id_for(&target.server_id, &target.room_id),
            None
        );
    });
}
