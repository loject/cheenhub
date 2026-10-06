use super::*;

#[test]
fn reassembles_fragmented_camera_key_frame() {
    let room_id = Uuid::new_v4();
    let sender_user_id = Uuid::new_v4();
    let mut reassembler = VideoFrameReassembler::default();
    let second = fragmented_video_datagram(FragmentFixture {
        room_id,
        sender_user_id,
        kind: MediaDatagramKind::CameraFrame,
        sequence: 7,
        flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
        total_len: 5,
        fragment_index: 1,
        fragment_count: 2,
        bytes: &[4, 5],
    });
    let first = fragmented_video_datagram(FragmentFixture {
        room_id,
        sender_user_id,
        kind: MediaDatagramKind::CameraFrame,
        sequence: 7,
        flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
        total_len: 5,
        fragment_index: 0,
        fragment_count: 2,
        bytes: &[1, 2, 3],
    });

    assert!(reassembler.push(second).is_none());
    let datagram = reassembler.push(first).expect("frame reassembles");

    assert_eq!(datagram.room_id, room_id);
    assert_eq!(datagram.sender_user_id, sender_user_id);
    assert_eq!(datagram.kind, MediaDatagramKind::CameraFrame);
    assert_eq!(datagram.sequence, 7);
    assert_eq!(datagram.flags, MEDIA_DATAGRAM_FLAG_KEY_FRAME);
    assert_eq!(datagram.payload, vec![1, 2, 3, 4, 5]);
}

struct FragmentFixture<'a> {
    room_id: Uuid,
    sender_user_id: Uuid,
    kind: MediaDatagramKind,
    sequence: u64,
    flags: u8,
    total_len: u32,
    fragment_index: u16,
    fragment_count: u16,
    bytes: &'a [u8],
}

fn fragmented_video_datagram(fragment: FragmentFixture<'_>) -> MediaDatagram {
    let mut payload = Vec::new();
    payload.extend_from_slice(&fragment.total_len.to_be_bytes());
    payload.extend_from_slice(&fragment.fragment_index.to_be_bytes());
    payload.extend_from_slice(&fragment.fragment_count.to_be_bytes());
    payload.extend_from_slice(fragment.bytes);

    MediaDatagram {
        kind: fragment.kind,
        codec: MediaCodec::Vp9,
        flags: fragment.flags | MEDIA_DATAGRAM_FLAG_FRAGMENTED,
        sequence: fragment.sequence,
        timestamp_us: 100,
        duration_us: 33_333,
        room_id: fragment.room_id,
        sender_user_id: fragment.sender_user_id,
        payload,
    }
}
