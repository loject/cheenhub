use super::*;
use cheenhub_contracts::media::MediaAudioSource;
use cheenhub_contracts::realtime::{ParticipantNetworkQualityUpdated, VoiceNetworkTargetKind};
use uuid::Uuid;

#[test]
fn decodes_participant_network_quality_event() {
    let event = ParticipantNetworkQualityUpdated {
        target_kind: VoiceNetworkTargetKind::Server,
        server_id: "server".to_owned(),
        room_id: "room".to_owned(),
        user_id: "user".to_owned(),
        session_id: Uuid::new_v4().to_string(),
        rtt_ms: 48,
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::ParticipantNetworkQualityUpdated),
        None,
        event.clone(),
    )
    .expect("event serializes");

    assert_eq!(participant_network_quality(envelope), Some(event));
}

#[test]
fn legacy_voice_frame_is_received_as_mono_microphone() {
    let packet = legacy_packet();

    let received = voice_frame(&packet.encode().unwrap()).unwrap();

    assert!(received.track_id.is_nil());
    assert_eq!(received.audio_source, MediaAudioSource::Microphone);
    assert_eq!(received.audio_channels, 1);
    assert_eq!(received.bytes, packet.payload);
}

#[test]
fn system_audio_frame_preserves_identity_channels_and_raw_opus() {
    let mut packet = legacy_packet();
    packet.track_id = Uuid::new_v4();
    packet.audio_source = Some(MediaAudioSource::System);
    packet.audio_channels = 2;

    let received = voice_frame(&packet.encode().unwrap()).unwrap();

    assert_eq!(received.track_id, packet.track_id);
    assert_eq!(received.audio_source, MediaAudioSource::System);
    assert_eq!(received.audio_channels, 2);
    assert_eq!(received.sender_user_id, packet.sender_user_id.to_string());
    assert_eq!(received.voice_session_id, None);
    assert_eq!(received.bytes, packet.payload);
}

#[test]
fn server_stamped_voice_sessions_distinguish_identical_system_audio_capture_ids() {
    let mut packet = legacy_packet();
    packet.track_id = Uuid::new_v4();
    packet.audio_source = Some(MediaAudioSource::System);
    packet.audio_channels = 2;
    let v2 = packet.encode().unwrap();
    let sessions = [Uuid::new_v4(), Uuid::new_v4()];

    for session_id in sessions {
        let mut v3 = v2[..82].to_vec();
        v3[4] = 3;
        v3.extend_from_slice(session_id.as_bytes());
        v3.extend_from_slice(&v2[82..]);

        let received = voice_frame(&v3).expect("server-stamped system audio decodes");

        assert_eq!(received.voice_session_id, Some(session_id));
        assert_eq!(received.track_id, packet.track_id);
        assert_eq!(received.sender_user_id, packet.sender_user_id.to_string());
        assert_eq!(received.audio_source, MediaAudioSource::System);
        assert_eq!(received.audio_channels, 2);
        assert_eq!(received.bytes, packet.payload);
    }
}

#[test]
fn video_frame_preserves_the_publishers_track_identity() {
    let mut packet = legacy_packet();
    packet.kind = MediaDatagramKind::CameraFrame;
    packet.codec = MediaCodec::Vp9;
    packet.track_id = Uuid::from_u128(3);

    let received = video_frame_from_datagram(packet.clone());

    assert_eq!(received.track_id, packet.track_id);
    assert_eq!(received.sender_user_id, packet.sender_user_id.to_string());
    assert_eq!(received.bytes, packet.payload);
}

#[test]
fn legacy_video_stop_without_session_id_still_decodes() {
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::VideoStreamEnded),
        None,
        serde_json::json!({
            "server_id": "server", "room_id": "room", "user_id": "peer",
            "source": "camera"
        }),
    )
    .unwrap();

    let event = video_stream_ended(envelope).expect("событие старого сервера декодируется");
    assert!(event.session_id.is_nil());
    assert_eq!(event.sender_user_id, "peer");
}

#[test]
fn video_stop_event_identifies_exact_voice_session() {
    let session_id = Uuid::from_u128(7);
    let event = VoiceVideoStreamEnded {
        session_id: session_id.to_string(),
        audio_track_id: None,
        server_id: "server".to_owned(),
        room_id: "room".to_owned(),
        user_id: "peer".to_owned(),
        source: cheenhub_contracts::realtime::VoiceVideoStreamSource::Camera,
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::VoiceChat,
        RealtimeKind::VoiceChat(VoiceChatKind::VideoStreamEnded),
        None,
        event,
    )
    .expect("stop event serializes");

    let received = video_stream_ended(envelope).expect("stop event decodes");
    assert_eq!(received.session_id, session_id);
    assert_eq!(received.sender_user_id, "peer");
}

fn legacy_packet() -> MediaDatagram {
    MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 1,
        timestamp_us: 10_000,
        duration_us: 10_000,
        room_id: Uuid::from_u128(1),
        sender_user_id: Uuid::from_u128(2),
        track_id: Uuid::nil(),
        audio_source: None,
        audio_channels: 0,
        payload: vec![1, 2, 3],
    }
}
