use bytes::Bytes;
use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramError, MediaDatagramKind};
use uuid::Uuid;

use super::MediaDatagramHeader;

#[test]
fn v5_microphone_must_belong_to_authenticated_voice_session() {
    use cheenhub_contracts::media::MediaAudioSource;
    let session = Uuid::new_v4();
    let another_session = Uuid::new_v4();
    let allowed = super::relay::authorized_audio_track;

    assert!(allowed(
        Some(MediaAudioSource::Microphone),
        session,
        session,
        5
    ));
    assert!(!allowed(
        Some(MediaAudioSource::Microphone),
        another_session,
        session,
        5
    ));
    assert!(!allowed(
        Some(MediaAudioSource::Microphone),
        Uuid::nil(),
        session,
        5
    ));
    assert!(!allowed(None, session, session, 5));
    assert!(allowed(
        Some(MediaAudioSource::System),
        another_session,
        session,
        5
    ));
    assert!(allowed(
        Some(MediaAudioSource::Microphone),
        another_session,
        session,
        4
    ));
}

#[test]
fn legacy_video_uses_authenticated_session_track_without_new_wire_version() {
    let mut packet = MediaDatagram::decode(&encoded_datagram(
        MediaDatagramKind::CameraFrame,
        MediaCodec::Vp9,
    ))
    .unwrap();
    packet.track_id = Uuid::nil();
    let encoded = packet.encode().unwrap();
    assert_eq!(encoded[4], 1, "старое видео использует v1");
    let header = MediaDatagramHeader::decode(&encoded).unwrap();
    let session_id = Uuid::new_v4();
    let normalized = super::relay::authorized_video_header(header, session_id, 4)
        .expect("старый клиент может отправлять legacy video");
    assert_eq!(normalized.track_id, session_id);
    assert!(super::relay::authorized_video_header(header, session_id, 5).is_none());
}

#[test]
fn relay_preserves_track_metadata_and_opus_when_replacing_sender() {
    let mut packet = MediaDatagram::decode(&encoded_datagram(
        MediaDatagramKind::VoiceFrame,
        MediaCodec::Opus,
    ))
    .unwrap();
    packet.track_id = Uuid::new_v4();
    packet.audio_source = Some(cheenhub_contracts::media::MediaAudioSource::System);
    packet.audio_channels = 2;
    let bytes = Bytes::from(packet.encode().unwrap());
    let sender = Uuid::new_v4();

    let header = MediaDatagramHeader::decode(&bytes).unwrap();
    assert_eq!(
        header.payload(&bytes).unwrap().as_ptr(),
        bytes[82..].as_ptr()
    );
    let relayed = header.into_relay_bytes(bytes, sender).unwrap();
    let decoded = MediaDatagram::decode(&relayed).unwrap();

    packet.sender_user_id = sender;
    assert_eq!(decoded, packet);
}

#[test]
fn relay_stamps_distinct_authenticated_sessions_without_changing_capture_or_payload() {
    use cheenhub_contracts::media::MediaAudioSource;

    let user_id = Uuid::new_v4();
    let capture_id = Uuid::new_v4();
    let sessions = [Uuid::new_v4(), Uuid::new_v4()];
    for source in [
        MediaDatagramKind::CameraFrame,
        MediaDatagramKind::ScreenFrame,
    ] {
        for session_id in sessions {
            let mut packet =
                MediaDatagram::decode(&encoded_datagram(source, MediaCodec::Vp9)).unwrap();
            packet.track_id = capture_id;
            let bytes = Bytes::from(packet.encode().unwrap());
            let header = MediaDatagramHeader::decode(&bytes).unwrap();
            let legacy = header.into_relay_bytes(bytes, user_id).unwrap();
            let stamped = super::fanout::stamp_relay_voice_session(&legacy, session_id).unwrap();
            let stamped_header = MediaDatagramHeader::decode(&stamped).unwrap();

            assert_eq!(legacy[4], 2, "старый клиент получает v2");
            assert_eq!(stamped[4], 3, "новый клиент получает v3");
            assert_eq!(stamped_header.voice_session_id, Some(session_id));
            assert_eq!(stamped_header.track_id, capture_id);
            assert_eq!(stamped_header.kind, source);
            assert_eq!(stamped_header.sender_user_id, user_id);
            assert_eq!(stamped_header.payload(&stamped).unwrap(), &packet.payload);
            assert_eq!(
                MediaDatagram::decode(&stamped).unwrap().payload,
                packet.payload
            );
        }
    }

    let mut audio = MediaDatagram::decode(&encoded_datagram(
        MediaDatagramKind::VoiceFrame,
        MediaCodec::Opus,
    ))
    .unwrap();
    audio.track_id = capture_id;
    audio.audio_source = Some(MediaAudioSource::System);
    let legacy = Bytes::from(audio.encode().unwrap());
    let first = super::fanout::stamp_relay_voice_session(&legacy, sessions[0]).unwrap();
    let second = super::fanout::stamp_relay_voice_session(&legacy, sessions[1]).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        MediaDatagramHeader::decode(&first)
            .unwrap()
            .voice_session_id,
        Some(sessions[0])
    );
    assert_eq!(
        MediaDatagramHeader::decode(&second)
            .unwrap()
            .voice_session_id,
        Some(sessions[1])
    );
}

#[test]
fn relay_refuses_client_supplied_server_session_metadata() {
    let bytes = Bytes::from(encoded_datagram(
        MediaDatagramKind::VoiceFrame,
        MediaCodec::Opus,
    ));
    let v3 = super::fanout::stamp_relay_voice_session(&bytes, Uuid::new_v4()).unwrap();
    let header = MediaDatagramHeader::decode(&v3).unwrap();
    assert_eq!(
        super::relay::prepare_relay_bytes(header, v3, Uuid::new_v4(), Uuid::new_v4(), 5),
        Err(MediaDatagramError::InvalidTrack)
    );
}

#[test]
fn legacy_v1_microphone_relay_uses_authenticated_session_and_preserves_opus() {
    use cheenhub_contracts::media::MediaAudioSource;

    let mut packet = MediaDatagram::decode(&encoded_datagram(
        MediaDatagramKind::VoiceFrame,
        MediaCodec::Opus,
    ))
    .unwrap();
    packet.track_id = Uuid::nil();
    packet.audio_source = None;
    packet.audio_channels = 0;
    let bytes = Bytes::from(packet.encode().unwrap());
    assert_eq!(bytes[4], 1);
    let header = MediaDatagramHeader::decode(&bytes).unwrap();
    let session_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();

    let relayed = super::relay::prepare_relay_bytes(header, bytes, sender_id, session_id, 4)
        .expect("legacy microphone relay must encode a session track");
    let decoded = MediaDatagram::decode(&relayed).unwrap();

    assert_eq!(relayed[4], 2);
    assert_eq!(decoded.sender_user_id, sender_id);
    assert_eq!(decoded.track_id, session_id);
    assert_eq!(decoded.audio_source, Some(MediaAudioSource::Microphone));
    assert_eq!(decoded.audio_channels, 1);
    assert_eq!(decoded.payload, packet.payload);
    assert_eq!(decoded.sequence, packet.sequence);
    assert_eq!(decoded.timestamp_us, packet.timestamp_us);
    assert_eq!(decoded.duration_us, packet.duration_us);
    assert_eq!(decoded.room_id, packet.room_id);
}

#[test]
fn legacy_v2_microphone_relay_replaces_capture_track_with_session_track() {
    let packet = MediaDatagram::decode(&encoded_datagram(
        MediaDatagramKind::VoiceFrame,
        MediaCodec::Opus,
    ))
    .unwrap();
    let old_track_id = packet.track_id;
    let session_id = Uuid::new_v4();
    let bytes = Bytes::from(packet.encode().unwrap());
    let header = MediaDatagramHeader::decode(&bytes).unwrap();

    let relayed =
        super::relay::prepare_relay_bytes(header, bytes, Uuid::new_v4(), session_id, 4).unwrap();
    let decoded = MediaDatagram::decode(&relayed).unwrap();

    assert_ne!(old_track_id, session_id);
    assert_eq!(decoded.track_id, session_id);
    assert_eq!(decoded.audio_source, packet.audio_source);
    assert_eq!(decoded.audio_channels, packet.audio_channels);
    assert_eq!(decoded.payload, packet.payload);
}

#[test]
fn legacy_system_audio_relay_keeps_capture_track_and_stereo_metadata() {
    use cheenhub_contracts::media::MediaAudioSource;

    let mut packet = MediaDatagram::decode(&encoded_datagram(
        MediaDatagramKind::VoiceFrame,
        MediaCodec::Opus,
    ))
    .unwrap();
    packet.audio_source = Some(MediaAudioSource::System);
    packet.audio_channels = 2;
    let bytes = Bytes::from(packet.encode().unwrap());
    let header = MediaDatagramHeader::decode(&bytes).unwrap();
    let sender_id = Uuid::new_v4();

    let relayed =
        super::relay::prepare_relay_bytes(header, bytes, sender_id, Uuid::new_v4(), 4).unwrap();
    let decoded = MediaDatagram::decode(&relayed).unwrap();

    packet.sender_user_id = sender_id;
    assert_eq!(decoded, packet);
}

#[test]
fn decoded_legacy_microphone_normalization_does_not_change_system_audio() {
    use cheenhub_contracts::media::MediaAudioSource;

    let mut microphone = MediaDatagram::decode(&encoded_datagram(
        MediaDatagramKind::VoiceFrame,
        MediaCodec::Opus,
    ))
    .unwrap();
    let session_id = Uuid::new_v4();
    assert!(super::relay::normalize_legacy_microphone_datagram(
        &mut microphone,
        session_id,
        4,
    ));
    assert_eq!(microphone.track_id, session_id);
    assert_eq!(microphone.audio_source, Some(MediaAudioSource::Microphone));

    let mut system = microphone.clone();
    system.audio_source = Some(MediaAudioSource::System);
    system.audio_channels = 2;
    system.track_id = Uuid::new_v4();
    let previous = system.clone();
    assert!(!super::relay::normalize_legacy_microphone_datagram(
        &mut system,
        session_id,
        4,
    ));
    assert_eq!(system, previous);
}

fn encoded_datagram(kind: MediaDatagramKind, codec: MediaCodec) -> Vec<u8> {
    MediaDatagram {
        track_id: Uuid::new_v4(),
        audio_source: (kind == MediaDatagramKind::VoiceFrame)
            .then_some(cheenhub_contracts::media::MediaAudioSource::Microphone),
        audio_channels: u8::from(kind == MediaDatagramKind::VoiceFrame),
        kind,
        codec,
        flags: 3,
        sequence: 42,
        timestamp_us: 123_456,
        duration_us: 20_000,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::new_v4(),
        payload: vec![1, 2, 3, 4],
    }
    .encode()
    .expect("datagram encodes")
}

#[test]
fn header_parser_reads_voice_fields_and_borrows_payload() {
    let bytes = encoded_datagram(MediaDatagramKind::VoiceFrame, MediaCodec::Opus);
    let expected = MediaDatagram::decode(&bytes).expect("owned datagram decodes");

    let header = MediaDatagramHeader::decode(&bytes).expect("header decodes");

    assert_eq!(header.kind, expected.kind);
    assert_eq!(header.codec, expected.codec);
    assert_eq!(header.flags, expected.flags);
    assert_eq!(header.sequence, expected.sequence);
    assert_eq!(header.timestamp_us, expected.timestamp_us);
    assert_eq!(header.duration_us, expected.duration_us);
    assert_eq!(header.room_id, expected.room_id);
    assert_eq!(header.sender_user_id, expected.sender_user_id);
    assert_eq!(header.payload_len, expected.payload.len());
    assert_eq!(header.payload(&bytes), Ok(expected.payload.as_slice()));
}

#[test]
fn header_parser_reads_video_kind_and_codec_without_copying_payload() {
    let bytes = encoded_datagram(MediaDatagramKind::ScreenFrame, MediaCodec::Vp9);

    let header = MediaDatagramHeader::decode(&bytes).expect("header decodes");

    assert_eq!(header.kind, MediaDatagramKind::ScreenFrame);
    assert_eq!(header.codec, MediaCodec::Vp9);
    let payload = header.payload(&bytes).expect("payload is borrowed");
    assert_eq!(payload, [1, 2, 3, 4]);
    assert_eq!(payload.as_ptr(), bytes[82..].as_ptr());
}

#[test]
fn header_parser_uses_declared_length_and_ignores_trailing_bytes() {
    let mut bytes = encoded_datagram(MediaDatagramKind::VoiceFrame, MediaCodec::Opus);
    let wire_len = bytes.len();
    bytes.extend_from_slice(&[8, 9]);

    let header = MediaDatagramHeader::decode(&bytes).expect("header accepts trailing bytes");

    assert_eq!(header.wire_len(), wire_len);
    assert_eq!(header.payload(&bytes), Ok(&bytes[82..wire_len]));
}

#[test]
fn header_parser_rejects_truncated_and_invalid_header_values() {
    let bytes = encoded_datagram(MediaDatagramKind::VoiceFrame, MediaCodec::Opus);

    assert_eq!(
        MediaDatagramHeader::decode(&bytes[..bytes.len() - 1]),
        Err(MediaDatagramError::Truncated)
    );

    let mut bad_magic = bytes.clone();
    bad_magic[0] = b'X';
    assert_eq!(
        MediaDatagramHeader::decode(&bad_magic),
        Err(MediaDatagramError::BadMagic)
    );

    let mut unknown_version = bytes.clone();
    unknown_version[4] = 99;
    assert_eq!(
        MediaDatagramHeader::decode(&unknown_version),
        Err(MediaDatagramError::UnknownVersion(99))
    );

    let mut unknown_kind = bytes.clone();
    unknown_kind[5] = 9;
    assert_eq!(
        MediaDatagramHeader::decode(&unknown_kind),
        Err(MediaDatagramError::UnknownKind(9))
    );

    let mut unknown_codec = bytes;
    unknown_codec[6] = 9;
    assert_eq!(
        MediaDatagramHeader::decode(&unknown_codec),
        Err(MediaDatagramError::UnknownCodec(9))
    );
}

#[test]
fn header_parser_rejects_buffer_shorter_than_declared_datagram() {
    let mut bytes = encoded_datagram(MediaDatagramKind::VoiceFrame, MediaCodec::Opus);
    bytes[60..64].copy_from_slice(&u32::MAX.to_be_bytes());

    assert_eq!(
        MediaDatagramHeader::decode(&bytes),
        Err(MediaDatagramError::Truncated)
    );
}

#[test]
fn relay_rewrites_only_sender_and_reuses_unique_bytes_for_opus() {
    let datagram = encoded_datagram(MediaDatagramKind::VoiceFrame, MediaCodec::Opus);
    let original = Bytes::from(datagram);
    let original_ptr = original.as_ptr();
    let authenticated_user_id = Uuid::new_v4();
    let header = MediaDatagramHeader::decode(&original).expect("header decodes");

    let relayed = header
        .into_relay_bytes(original, authenticated_user_id)
        .expect("relay buffer is prepared");

    assert_eq!(relayed.as_ptr(), original_ptr);
    assert_eq!(&relayed[44..60], authenticated_user_id.as_bytes());
    assert_eq!(&relayed[82..], [1, 2, 3, 4]);
}

#[test]
fn relay_falls_back_to_one_full_copy_for_shared_vp9_bytes() {
    let original = Bytes::from(encoded_datagram(
        MediaDatagramKind::ScreenFrame,
        MediaCodec::Vp9,
    ));
    let shared = original.clone();
    let forged_sender_id = original[44..60].to_vec();
    let original_track_id = original[64..80].to_vec();
    let authenticated_user_id = Uuid::new_v4();
    let header = MediaDatagramHeader::decode(&original).expect("header decodes");

    let relayed = header
        .into_relay_bytes(original, authenticated_user_id)
        .expect("relay buffer is prepared");

    assert_eq!(&shared[44..60], forged_sender_id);
    assert_eq!(&shared[64..80], original_track_id);
    assert_eq!(&relayed[44..60], authenticated_user_id.as_bytes());
    assert_eq!(&relayed[64..80], original_track_id);
    assert_eq!(&relayed[82..], [1, 2, 3, 4]);
}
