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
        track_id: Uuid::from_u128(3),
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
        track_id: Uuid::from_u128(3),
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

#[test]
fn same_user_video_tracks_with_identical_sequences_reassemble_independently() {
    let room_id = Uuid::new_v4();
    let sender_user_id = Uuid::new_v4();
    let first_track = Uuid::new_v4();
    let second_track = Uuid::new_v4();
    let mut reassembler = VideoFrameReassembler::default();
    let fragment = |track_id, fragment_index, bytes: &'static [u8]| {
        fragmented_video_datagram(FragmentFixture {
            room_id,
            sender_user_id,
            kind: MediaDatagramKind::CameraFrame,
            sequence: 9,
            flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
            track_id,
            total_len: 4,
            fragment_index,
            fragment_count: 2,
            bytes,
        })
    };

    assert!(
        reassembler
            .push(fragment(first_track, 0, &[1, 2]))
            .is_none()
    );
    assert!(
        reassembler
            .push(fragment(second_track, 0, &[5, 6]))
            .is_none()
    );
    let first = reassembler
        .push(fragment(first_track, 1, &[3, 4]))
        .expect("первый поток собирается отдельно");
    let second = reassembler
        .push(fragment(second_track, 1, &[7, 8]))
        .expect("второй поток собирается отдельно");

    assert_eq!(first.track_id, first_track);
    assert_eq!(first.payload, [1, 2, 3, 4]);
    assert_eq!(second.track_id, second_track);
    assert_eq!(second.payload, [5, 6, 7, 8]);
}

struct FragmentFixture<'a> {
    room_id: Uuid,
    sender_user_id: Uuid,
    kind: MediaDatagramKind,
    sequence: u64,
    flags: u8,
    track_id: Uuid,
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
        track_id: fragment.track_id,
        audio_source: None,
        audio_channels: 0,
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
