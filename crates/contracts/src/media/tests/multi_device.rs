use super::super::*;

fn tracked_packet() -> MediaDatagram {
    MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 42,
        timestamp_us: 80_000,
        duration_us: 20_000,
        room_id: Uuid::from_u128(1),
        sender_user_id: Uuid::from_u128(2),
        track_id: Uuid::from_u128(4),
        audio_source: Some(MediaAudioSource::Microphone),
        audio_channels: 1,
        payload: vec![7, 8, 9],
    }
}

#[test]
fn unreleased_wire_versions_are_rejected() {
    let bytes = tracked_packet().encode().unwrap();
    for version in [4_u8, 5, 6] {
        let mut unsupported = bytes.clone();
        unsupported[4] = version;
        assert_eq!(
            MediaDatagram::decode(&unsupported),
            Err(MediaDatagramError::UnknownVersion(version)),
            "version={version}"
        );
    }
}

#[test]
fn v2_identifies_track_without_redundant_session_field() {
    let packet = tracked_packet();
    let bytes = packet.encode().unwrap();

    assert_eq!(bytes[4], 2);
    assert_eq!(bytes.len(), 82 + packet.payload.len());
    assert_eq!(&bytes[64..80], packet.track_id.as_bytes());
    assert_eq!(&bytes[82..], packet.payload);
    assert_eq!(MediaDatagram::decode(&bytes).unwrap(), packet);
}

#[test]
fn v3_carries_authenticated_voice_session_and_preserves_payload() {
    let packet = tracked_packet();
    let session_id = Uuid::from_u128(55);
    let v2 = packet.encode().unwrap();
    let mut v3 = v2[..82].to_vec();
    v3[4] = 3;
    v3.extend_from_slice(session_id.as_bytes());
    v3.extend_from_slice(&v2[82..]);

    let header = MediaDatagramHeader::decode(&v3).unwrap();
    assert_eq!(header.voice_session_id, Some(session_id));
    assert_eq!(header.wire_len(), 98 + packet.payload.len());
    assert_eq!(header.payload(&v3).unwrap(), packet.payload);
    assert_eq!(MediaDatagram::decode(&v3).unwrap(), packet);
    assert_eq!(
        MediaDatagramHeader::decode(&v2).unwrap().voice_session_id,
        None
    );
}

#[test]
fn v3_rejects_missing_authenticated_voice_session() {
    let packet = tracked_packet();
    let v2 = packet.encode().unwrap();
    let mut v3 = v2[..82].to_vec();
    v3[4] = 3;
    v3.extend_from_slice(Uuid::nil().as_bytes());
    v3.extend_from_slice(&v2[82..]);

    assert_eq!(
        MediaDatagramHeader::decode(&v3),
        Err(MediaDatagramError::InvalidTrack)
    );
}

#[test]
fn legacy_v1_decodes_without_track() {
    let mut packet = tracked_packet();
    packet.track_id = Uuid::nil();
    packet.audio_source = None;
    packet.audio_channels = 0;
    let bytes = packet.encode().unwrap();

    assert_eq!(bytes[4], 1);
    assert_eq!(MediaDatagram::decode(&bytes).unwrap(), packet);
}
