//! Регрессии идентификации отдельного голосового присутствия одного аккаунта.

use cheenhub_contracts::realtime::VoicePresenceEvicted;
use uuid::Uuid;

use super::*;

#[test]
fn server_without_confirmed_voice_session_requires_upgrade() {
    use cheenhub_contracts::realtime::RejectionCode;

    let target = VoiceRoomTarget::server("server".into(), "room".into(), "Общая".into());
    let response = snapshot(&target, vec![participant(CURRENT_USER_ID)]);
    let error = super::super::actions::confirmed_voice_session_id(&response)
        .expect_err("клиент не должен выдумывать UUID голосовой сессии");

    assert_eq!(
        error.rejection_code(),
        Some(RejectionCode::VoiceClientUpgradeRequired)
    );
    assert!(error.user_message().unwrap().contains("Обновите CheenHub"));
}

#[test]
fn join_adds_own_session_even_if_another_device_of_same_user_is_present() {
    let another_device = participant(CURRENT_USER_ID);
    let own_session_id = Uuid::new_v4().to_string();
    let mut participants = vec![another_device.clone()];

    super::super::actions::ensure_current_user_present(
        &mut participants,
        &current_user(),
        true,
        &own_session_id,
    );

    assert_eq!(participants.len(), 2);
    assert_eq!(participants[0].session_id, another_device.session_id);
    assert_eq!(participants[1].session_id, own_session_id);
}

#[test]
fn late_precise_eviction_replaces_kick_after_snapshot_removed_own_session() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server".into(), "room".into(), "Общая".into());
        let own_session_id = Uuid::new_v4();
        let mut own = participant(CURRENT_USER_ID);
        own.session_id = own_session_id.to_string();
        let joined_at = own.joined_at.clone();
        let (handle, _) = voice_handle(VoiceConnectionState::Connected {
            target: target.clone(),
            participants: vec![own],
        });
        let mut media_session = handle.media_session;
        media_session.set(Some(super::super::VoiceMediaSession {
            target: target.clone(),
            session_id: own_session_id,
        }));

        handle.apply_snapshot(snapshot(&target, vec![participant(PEER_USER_ID)]));
        assert_eq!((handle.kicked_from_room)(), Some(target.room_name.clone()));
        handle.apply_eviction(
            serde_json::from_value(serde_json::json!({
                "session_id": own_session_id,
                "joined_at": joined_at,
                "target_kind": "server",
                "server_id": target.server_id,
                "room_id": target.room_id,
                "reason": "same_auth_session_takeover"
            }))
            .unwrap(),
        );

        assert_eq!((handle.kicked_from_room)(), None);
        assert_eq!(
            (handle.eviction_notice)(),
            Some(
                cheenhub_contracts::realtime::VoicePresenceEvictionReason::SameAuthSessionTakeover
            )
        );
    });
}

#[test]
fn late_precise_eviction_replaces_generic_notice_after_same_user_snapshot() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server".into(), "room".into(), "Общая".into());
        let own_session_id = Uuid::new_v4();
        let mut own = participant(CURRENT_USER_ID);
        own.session_id = own_session_id.to_string();
        let joined_at = own.joined_at.clone();
        let (handle, _) = voice_handle(VoiceConnectionState::Connected {
            target: target.clone(),
            participants: vec![own],
        });
        let mut media_session = handle.media_session;
        media_session.set(Some(super::super::VoiceMediaSession {
            target: target.clone(),
            session_id: own_session_id,
        }));

        handle.apply_snapshot(snapshot(&target, vec![participant(CURRENT_USER_ID)]));
        handle.apply_eviction(
            serde_json::from_value(serde_json::json!({
                "session_id": own_session_id,
                "joined_at": joined_at,
                "target_kind": "server",
                "server_id": target.server_id,
                "room_id": target.room_id,
                "reason": "same_auth_session_takeover"
            }))
            .unwrap(),
        );

        assert_eq!(
            (handle.eviction_notice)(),
            Some(
                cheenhub_contracts::realtime::VoicePresenceEvictionReason::SameAuthSessionTakeover
            )
        );
    });
}

#[test]
fn delayed_eviction_of_old_presence_cannot_disconnect_rejoined_transport() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server".into(), "room".into(), "Общая".into());
        let session_id = Uuid::new_v4();
        let mut own = participant(CURRENT_USER_ID);
        own.session_id = session_id.to_string();
        own.joined_at = "2026-10-31T10:00:02Z".into();
        let (handle, _) = voice_handle(VoiceConnectionState::Connected {
            target: target.clone(),
            participants: vec![own],
        });
        let mut media_session = handle.media_session;
        media_session.set(Some(super::super::VoiceMediaSession {
            target: target.clone(),
            session_id,
        }));
        let event: VoicePresenceEvicted = serde_json::from_value(serde_json::json!({
            "session_id": session_id,
            "joined_at": "2026-10-31T10:00:01Z",
            "target_kind": "server",
            "server_id": target.server_id,
            "room_id": target.room_id,
            "reason": "same_auth_session_takeover"
        }))
        .unwrap();

        handle.apply_eviction(event);

        assert!(matches!(
            handle.state(),
            VoiceConnectionState::Connected { .. }
        ));
        assert_eq!((handle.eviction_notice)(), None);
    });
}

#[test]
fn delayed_eviction_during_join_cannot_cancel_new_presence() {
    with_dioxus_runtime(|| {
        let target = VoiceRoomTarget::server("server".into(), "room".into(), "Общая".into());
        let session_id = Uuid::new_v4().to_string();
        let (handle, _) = voice_handle(VoiceConnectionState::Connecting {
            target: target.clone(),
        });
        let event: VoicePresenceEvicted = serde_json::from_value(serde_json::json!({
            "session_id": session_id,
            "joined_at": "2026-10-31T10:00:01Z",
            "target_kind": "server",
            "server_id": target.server_id,
            "room_id": target.room_id,
            "reason": "another_device_transition"
        }))
        .unwrap();
        handle.apply_eviction(event);

        let mut own = participant(CURRENT_USER_ID);
        own.session_id = session_id.clone();
        own.joined_at = "2026-10-31T10:00:02Z".into();
        handle.complete_join(
            &target,
            ACTIVE_JOIN_GENERATION,
            JoinedVoiceRoom {
                snapshot: snapshot(&target, vec![own]),
                self_session_id: session_id,
                audio_bitrate_bps: 48_000,
            },
        );

        assert!(matches!(
            handle.state(),
            VoiceConnectionState::Connected { .. }
        ));
        assert_eq!((handle.eviction_notice)(), None);
    });
}
