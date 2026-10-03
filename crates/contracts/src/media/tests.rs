use super::*;

#[test]
fn media_datagram_round_trips() {
    let datagram = MediaDatagram {
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
    unknown_version[4] = 2;
    assert_eq!(
        MediaDatagram::decode(&unknown_version),
        Err(MediaDatagramError::UnknownVersion(2))
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
