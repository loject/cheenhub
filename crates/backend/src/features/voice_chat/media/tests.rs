use bytes::Bytes;
use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramError, MediaDatagramKind};
use uuid::Uuid;

use super::MediaDatagramHeader;

fn encoded_datagram(kind: MediaDatagramKind, codec: MediaCodec) -> Vec<u8> {
    MediaDatagram {
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
    assert_eq!(payload.as_ptr(), bytes[64..].as_ptr());
}

#[test]
fn header_parser_uses_declared_length_and_ignores_trailing_bytes() {
    let mut bytes = encoded_datagram(MediaDatagramKind::VoiceFrame, MediaCodec::Opus);
    let wire_len = bytes.len();
    bytes.extend_from_slice(&[8, 9]);

    let header = MediaDatagramHeader::decode(&bytes).expect("header accepts trailing bytes");

    assert_eq!(header.wire_len(), wire_len);
    assert_eq!(header.payload(&bytes), Ok(&bytes[64..wire_len]));
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
    unknown_version[4] = 2;
    assert_eq!(
        MediaDatagramHeader::decode(&unknown_version),
        Err(MediaDatagramError::UnknownVersion(2))
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
    assert_eq!(&relayed[64..], [1, 2, 3, 4]);
}

#[test]
fn relay_falls_back_to_one_full_copy_for_shared_vp9_bytes() {
    let original = Bytes::from(encoded_datagram(
        MediaDatagramKind::ScreenFrame,
        MediaCodec::Vp9,
    ));
    let shared = original.clone();
    let forged_sender_id = original[44..60].to_vec();
    let authenticated_user_id = Uuid::new_v4();
    let header = MediaDatagramHeader::decode(&original).expect("header decodes");

    let relayed = header
        .into_relay_bytes(original, authenticated_user_id)
        .expect("relay buffer is prepared");

    assert_eq!(&shared[44..60], forged_sender_id);
    assert_eq!(&relayed[44..60], authenticated_user_id.as_bytes());
    assert_eq!(&relayed[64..], [1, 2, 3, 4]);
}
