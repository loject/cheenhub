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
    assert_eq!(received.bytes, packet.payload);
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
