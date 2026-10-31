use super::super::*;
use serde_json::json;
use uuid::Uuid;

#[test]
fn join_without_flags_defaults_to_no_confirmation() {
    let join: JoinVoiceRoom = serde_json::from_value(json!({
        "server_id": "server", "room_id": "room"
    }))
    .unwrap();

    assert!(!join.confirm_disconnect_other_devices);
    assert_eq!(join.voice_protocol_version, 0);
    assert_eq!(join.device_kind, VoiceDeviceKind::Unknown);

    let direct: JoinDirectMessageVoiceRoom = serde_json::from_value(json!({
        "conversation_id": "conversation"
    }))
    .unwrap();

    assert!(!direct.confirm_disconnect_other_devices);
    assert_eq!(direct.voice_protocol_version, 0);
    assert_eq!(direct.device_kind, VoiceDeviceKind::Unknown);
}

#[test]
fn legacy_snapshot_without_session_fields_remains_readable() {
    let snapshot: VoiceRoomSnapshot = serde_json::from_value(json!({
        "server_id": "server",
        "room_id": "room",
        "participants": [{
            "user_id": "legacy-user",
            "nickname": "Старый клиент",
            "avatar_url": null,
            "joined_at": "2026-10-30T00:00:00Z"
        }]
    }))
    .expect("старый снимок голосовой комнаты должен декодироваться");

    assert_eq!(snapshot.self_session_id, None);
    assert!(snapshot.participants[0].session_id.is_empty());
    let encoded = serde_json::to_value(snapshot).unwrap();
    assert!(encoded.get("self_session_id").is_none());
}

#[test]
fn own_session_extends_snapshot_without_changing_response_shape() {
    let session_id = Uuid::new_v4().to_string();
    let snapshot: VoiceRoomSnapshot = serde_json::from_value(json!({
        "server_id": "server",
        "room_id": "room",
        "self_session_id": session_id,
        "participants": []
    }))
    .unwrap();

    assert_eq!(
        snapshot.self_session_id.as_deref(),
        Some(session_id.as_str())
    );
    assert!(
        serde_json::to_value(snapshot)
            .unwrap()
            .get("snapshot")
            .is_none()
    );
}

#[test]
fn join_snapshot_distinguishes_own_and_other_device_sessions() {
    let own_session = Uuid::new_v4().to_string();
    let another_session = Uuid::new_v4().to_string();
    let result: VoiceRoomSnapshot = serde_json::from_value(json!({
        "server_id": "server",
        "room_id": "room",
        "self_session_id": own_session,
        "participants": [
            {
                "session_id": another_session,
                "device_kind": "mobile",
                "user_id": "same-user",
                "nickname": "Чингиз",
                "avatar_url": null,
                "joined_at": "2026-10-30T00:00:00Z"
            },
            {
                "session_id": own_session,
                "device_kind": "desktop",
                "user_id": "same-user",
                "nickname": "Чингиз",
                "avatar_url": null,
                "joined_at": "2026-10-30T00:00:01Z"
            }
        ]
    }))
    .unwrap();

    assert_eq!(
        result.self_session_id.as_deref(),
        Some(own_session.as_str())
    );
    assert_eq!(result.participants.len(), 2);
    assert_eq!(result.participants[0].session_id, another_session);
    assert_eq!(result.participants[1].session_id, own_session);
    assert_eq!(result.participants[0].user_id, "same-user");
    assert_eq!(result.participants[1].user_id, "same-user");
    assert_eq!(result.participants[0].device_kind, VoiceDeviceKind::Mobile);
    assert_eq!(result.participants[1].device_kind, VoiceDeviceKind::Desktop);
}

#[test]
fn voice_conflict_has_typed_rejection() {
    let code: RejectionCode =
        serde_json::from_value(json!("voice_device_confirmation_required")).unwrap();

    assert_eq!(code, RejectionCode::VoiceDeviceConfirmationRequired);
    assert_eq!(
        serde_json::to_value(RejectionCode::VoiceClientUpgradeRequired).unwrap(),
        json!("voice_client_upgrade_required")
    );
}

#[test]
fn same_auth_session_conflict_has_distinct_code() {
    let code: RejectionCode =
        serde_json::from_value(json!("voice_session_takeover_confirmation_required")).unwrap();

    assert_eq!(
        code,
        RejectionCode::VoiceSessionTakeoverConfirmationRequired
    );
    assert_ne!(code, RejectionCode::VoiceDeviceConfirmationRequired);
}

#[test]
fn eviction_identifies_one_session() {
    let session_id = Uuid::new_v4().to_string();
    let evicted = VoicePresenceEvicted {
        session_id: session_id.clone(),
        joined_at: "2026-10-31T10:00:00Z".to_owned(),
        target_kind: VoiceNetworkTargetKind::Server,
        server_id: "server".to_owned(),
        room_id: "room".to_owned(),
        reason: VoicePresenceEvictionReason::AnotherDeviceTransition,
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::VoicePresenceEvicted),
        None,
        evicted.clone(),
    )
    .unwrap();

    assert!(envelope.has_matching_module_kind());
    let decoded: VoicePresenceEvicted = serde_json::from_value(envelope.payload).unwrap();
    assert_eq!(decoded, evicted);
    assert_eq!(decoded.session_id, session_id);
}

#[test]
fn same_auth_session_eviction_has_distinct_reason() {
    let reason: VoicePresenceEvictionReason =
        serde_json::from_value(json!("same_auth_session_takeover")).unwrap();

    assert_eq!(reason, VoicePresenceEvictionReason::SameAuthSessionTakeover);
    assert_ne!(reason, VoicePresenceEvictionReason::AnotherDeviceTransition);
}

#[test]
fn media_lifecycle_events_identify_exact_voice_session() {
    let session_id = Uuid::new_v4().to_string();
    let ended: VoiceVideoStreamEnded = serde_json::from_value(json!({
        "server_id": "server", "room_id": "room", "user_id": "user",
        "session_id": session_id, "source": "screen_share"
    }))
    .unwrap();
    let quality: ParticipantNetworkQualityUpdated = serde_json::from_value(json!({
        "target_kind": "server", "server_id": "server", "room_id": "room",
        "user_id": "user", "session_id": session_id, "rtt_ms": 42
    }))
    .unwrap();

    assert_eq!(ended.session_id, session_id);
    assert_eq!(quality.session_id, session_id);
}
