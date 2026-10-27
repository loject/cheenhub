use super::*;

#[test]
fn audio_tracks_preserve_independent_identity_and_source() {
    let microphone = MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 1,
        timestamp_us: 20_000,
        duration_us: 20_000,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::new_v4(),
        track_id: Uuid::new_v4(),
        audio_source: Some(MediaAudioSource::Microphone),
        audio_channels: 1,
        payload: vec![1, 2, 3],
    };
    let system = MediaDatagram {
        track_id: Uuid::new_v4(),
        audio_source: Some(MediaAudioSource::System),
        audio_channels: 2,
        payload: vec![4, 5, 6],
        ..microphone.clone()
    };

    let microphone_bytes = microphone.encode().unwrap();
    let system_bytes = system.encode().unwrap();

    assert_eq!(
        MediaDatagram::decode(&microphone_bytes).unwrap(),
        microphone
    );
    assert_eq!(MediaDatagram::decode(&system_bytes).unwrap(), system);
    assert_eq!(microphone_bytes[4], 2);
    assert_eq!(&system_bytes[82..], system.payload);
    assert_ne!(microphone.track_id, system.track_id);
}

#[test]
fn audio_track_rejects_missing_identity_and_invalid_channels() {
    let mut frame = MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 1,
        timestamp_us: 20_000,
        duration_us: 20_000,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::nil(),
        track_id: Uuid::nil(),
        audio_source: Some(MediaAudioSource::System),
        audio_channels: 2,
        payload: vec![1],
    };
    assert!(frame.encode().is_err());

    frame.track_id = Uuid::new_v4();
    frame.audio_channels = 3;
    assert!(frame.encode().is_err());

    frame.audio_channels = 2;
    let bytes = frame.encode().unwrap();
    for length in 64..82 {
        assert!(
            MediaDatagram::decode(&bytes[..length]).is_err(),
            "length={length}"
        );
    }
}

#[test]
fn legacy_voice_bytes_decode_without_track_metadata_and_reencode_unchanged() {
    let mut bytes = Vec::from(*b"CHUB");
    bytes.extend_from_slice(&[1, 1, 1, 0]);
    bytes.extend_from_slice(&42_u64.to_be_bytes());
    bytes.extend_from_slice(&123_456_u64.to_be_bytes());
    bytes.extend_from_slice(&20_000_u32.to_be_bytes());
    bytes.extend_from_slice(Uuid::from_u128(1).as_bytes());
    bytes.extend_from_slice(Uuid::from_u128(2).as_bytes());
    bytes.extend_from_slice(&3_u32.to_be_bytes());
    bytes.extend_from_slice(&[1, 2, 3]);

    let decoded = MediaDatagram::decode(&bytes).unwrap();

    assert!(decoded.track_id.is_nil());
    assert_eq!(decoded.audio_source, None);
    assert_eq!(decoded.audio_channels, 0);
    assert_eq!(decoded.payload, [1, 2, 3]);
    assert_eq!(decoded.encode().unwrap(), bytes);
}

#[test]
fn tracked_audio_rejects_invalid_wire_metadata_before_exposing_payload() {
    let packet = MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 1,
        timestamp_us: 10_000,
        duration_us: 10_000,
        room_id: Uuid::from_u128(1),
        sender_user_id: Uuid::from_u128(2),
        track_id: Uuid::from_u128(3),
        audio_source: Some(MediaAudioSource::System),
        audio_channels: 2,
        payload: vec![1, 2],
    };
    let valid = packet.encode().unwrap();
    let mut missing_identity = valid.clone();
    missing_identity[64..80].fill(0);
    let mut unknown_source = valid.clone();
    unknown_source[80] = 99;
    let mut missing_source = valid.clone();
    missing_source[80] = 0;
    let mut invalid_channels = valid.clone();
    invalid_channels[81] = 3;
    let mut audio_on_video = valid;
    audio_on_video[5] = 2;

    for bytes in [
        missing_identity,
        unknown_source,
        missing_source,
        invalid_channels,
        audio_on_video,
    ] {
        assert_eq!(
            MediaDatagram::decode(&bytes),
            Err(MediaDatagramError::InvalidTrack)
        );
        assert_eq!(
            MediaDatagramHeader::decode(&bytes),
            Err(MediaDatagramError::InvalidTrack)
        );
    }
}

#[test]
fn media_datagram_round_trips() {
    let datagram = MediaDatagram {
        track_id: uuid::Uuid::nil(),
        audio_source: None,
        audio_channels: 0,
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 42,
        timestamp_us: 123_456,
        duration_us: 20_000,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::new_v4(),
        payload: vec![1, 2, 3, 4],
    };

    let encoded = datagram.encode().expect("datagram encodes");
    let decoded = MediaDatagram::decode(&encoded).expect("datagram decodes");

    assert_eq!(decoded, datagram);
}

#[test]
fn screen_media_datagram_round_trips() {
    let datagram = MediaDatagram {
        track_id: uuid::Uuid::nil(),
        audio_source: None,
        audio_channels: 0,
        kind: MediaDatagramKind::ScreenFrame,
        codec: MediaCodec::Vp9,
        flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
        sequence: 84,
        timestamp_us: 654_321,
        duration_us: 33_333,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::new_v4(),
        payload: vec![9, 8, 7, 6],
    };

    let encoded = datagram.encode().expect("datagram encodes");
    let decoded = MediaDatagram::decode(&encoded).expect("datagram decodes");

    assert_eq!(decoded, datagram);
}

#[test]
fn camera_media_datagram_round_trips() {
    let datagram = MediaDatagram {
        track_id: uuid::Uuid::nil(),
        audio_source: None,
        audio_channels: 0,
        kind: MediaDatagramKind::CameraFrame,
        codec: MediaCodec::Vp9,
        flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
        sequence: 21,
        timestamp_us: 456_123,
        duration_us: 41_667,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::new_v4(),
        payload: vec![5, 6, 7, 8],
    };

    let encoded = datagram.encode().expect("datagram encodes");
    let decoded = MediaDatagram::decode(&encoded).expect("datagram decodes");

    assert_eq!(decoded, datagram);
}

#[test]
fn media_datagram_rejects_truncated_payload() {
    let datagram = MediaDatagram {
        track_id: uuid::Uuid::nil(),
        audio_source: None,
        audio_channels: 0,
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 1,
        timestamp_us: 1,
        duration_us: 20_000,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::nil(),
        payload: vec![1, 2, 3],
    };
    let mut encoded = datagram.encode().expect("datagram encodes");
    encoded.pop();

    assert_eq!(
        MediaDatagram::decode(&encoded),
        Err(MediaDatagramError::Truncated)
    );
}

#[test]
fn media_datagram_rejects_unknown_version_kind_and_codec() {
    let datagram = MediaDatagram {
        track_id: uuid::Uuid::nil(),
        audio_source: None,
        audio_channels: 0,
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: 1,
        timestamp_us: 1,
        duration_us: 20_000,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::nil(),
        payload: vec![],
    };
    let encoded = datagram.encode().expect("datagram encodes");

    let mut unknown_version = encoded.clone();
    unknown_version[4] = 99;
    assert_eq!(
        MediaDatagram::decode(&unknown_version),
        Err(MediaDatagramError::UnknownVersion(99))
    );

    let mut unknown_kind = encoded.clone();
    unknown_kind[5] = 9;
    assert_eq!(
        MediaDatagram::decode(&unknown_kind),
        Err(MediaDatagramError::UnknownKind(9))
    );

    let mut unknown_codec = encoded;
    unknown_codec[6] = 9;
    assert_eq!(
        MediaDatagram::decode(&unknown_codec),
        Err(MediaDatagramError::UnknownCodec(9))
    );
}
