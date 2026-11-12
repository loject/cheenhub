//! Проверки политики видеопубликаций и подсчёта активных видеоисточников.

mod activity;
mod burst;
mod fragmentation;
mod screen_dimensions;
pub(crate) mod support;

use super::*;
use crate::features::voice_chat::media::MediaDatagramHeader;
use cheenhub_contracts::{
    media::{MEDIA_DATAGRAM_FLAG_FRAGMENTED, MediaCodec, MediaDatagram},
    video_presets::{CAMERA_BITRATE_LIMIT, SCREEN_SHARE_BITRATE_LIMIT},
};

#[test]
fn screen_policy_accepts_both_base_resolutions() {
    for (sequence, width, height) in [(1, 1280, 720), (2, 1920, 1080)] {
        let mut tracker = VideoPublicationTracker::default();
        let mut datagram = video_datagram(sequence, true, width, height);
        datagram.kind = MediaDatagramKind::ScreenFrame;
        assert_eq!(
            tracker.inspect_at(
                Uuid::new_v4(),
                &datagram,
                SCREEN_SHARE_BITRATE_LIMIT,
                Instant::now(),
            ),
            VideoAdmission::Forward
        );
    }
}

#[test]
fn camera_policy_accepts_4k_independent_of_preset_dimensions() {
    let mut tracker = VideoPublicationTracker::default();
    let datagram = video_datagram(1, true, 3840, 2160);
    assert_eq!(
        tracker.inspect_at(
            Uuid::new_v4(),
            &datagram,
            CAMERA_BITRATE_LIMIT,
            Instant::now(),
        ),
        VideoAdmission::Forward
    );
}

#[test]
fn video_policy_accepts_key_frame_from_borrowed_wire_payload() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let datagram = video_datagram(1, true, 1280, 720);
    let wire = datagram.encode().expect("datagram encodes");
    let header = MediaDatagramHeader::decode(&wire).expect("header decodes");
    let payload = header.payload(&wire).expect("payload is borrowed");

    assert_eq!(
        tracker.inspect_header_at(session_id, &header, payload, CAMERA_BITRATE_LIMIT, now,),
        VideoAdmission::Forward
    );
    assert_eq!(tracker.publications.len(), 1);
    assert!(tracker.publications[0].has_key_frame);
}

#[test]
fn fragmented_frame_is_counted_once() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let first = fragmented(video_datagram(1, true, 1280, 720), 0, 2);
    let mut second = fragmented(video_datagram(1, true, 1280, 720), 1, 2);
    second.room_id = first.room_id;
    assert_eq!(
        tracker.inspect_at(session_id, &first, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session_id, &second, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert!(tracker.publications[0].has_key_frame);
}

#[test]
fn fragmented_frame_rejects_bytes_exceeding_reserved_frame_size() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let first = fragmented(video_datagram(1, true, 1280, 720), 0, 2);
    let mut second = fragmented(video_datagram(1, true, 1280, 720), 1, 2);
    second.room_id = first.room_id;
    let declared = u32::from_be_bytes(first.payload[..4].try_into().unwrap()) as usize;
    second.payload.extend(std::iter::repeat_n(0, declared));

    assert_eq!(
        tracker.inspect_at(session_id, &first, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session_id, &second, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
}

#[test]
fn fragmented_frame_rejects_changed_declared_length() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let first = fragmented(video_datagram(1, true, 1280, 720), 0, 2);
    let mut second = fragmented(video_datagram(1, true, 1280, 720), 1, 2);
    second.room_id = first.room_id;
    second.payload[..4].copy_from_slice(&100_000_u32.to_be_bytes());
    second.payload.extend(std::iter::repeat_n(0, 1_000));

    assert_eq!(
        tracker.inspect_at(session_id, &first, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session_id, &second, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
}

#[test]
fn camera_accepts_high_fps_when_bitrate_is_within_budget() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let started = Instant::now();
    let mut datagram = video_datagram(1, true, 1280, 720);
    let room_id = datagram.room_id;
    assert_eq!(
        tracker.inspect_at(session_id, &datagram, CAMERA_BITRATE_LIMIT, started),
        VideoAdmission::Forward
    );
    datagram.flags = 0;
    datagram.payload = vec![0; 100];
    datagram.room_id = room_id;
    for sequence in 2..=121 {
        datagram.sequence = sequence;
        assert_eq!(
            tracker.inspect_at(
                session_id,
                &datagram,
                CAMERA_BITRATE_LIMIT,
                started + Duration::from_millis(sequence * 8),
            ),
            VideoAdmission::Forward,
            "frame {sequence}"
        );
    }
}

#[test]
fn camera_allows_two_second_burst_then_rejects_excess_bytes() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let mut datagram = video_datagram(1, true, 1280, 720);
    let room_id = datagram.room_id;
    datagram.payload.resize(100_000, 0);

    assert_eq!(
        tracker.inspect_at(session_id, &datagram, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );

    datagram.sequence = 2;
    datagram.flags = 0;
    datagram.payload.resize(75_000, 0);
    assert_eq!(
        tracker.inspect_at(session_id, &datagram, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );

    datagram.sequence = 3;
    datagram.payload.resize(1, 0);
    assert!(matches!(
        tracker.inspect_at(session_id, &datagram, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
    ));

    let mut recovery = video_datagram(4, true, 1280, 720);
    recovery.room_id = room_id;
    assert_eq!(
        tracker.inspect_at(
            session_id,
            &recovery,
            CAMERA_BITRATE_LIMIT,
            now + Duration::from_secs(1)
        ),
        VideoAdmission::Forward
    );
}

#[test]
fn screen_burst_uses_its_own_one_point_five_mbps_budget() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let mut datagram = video_datagram(1, true, 3840, 2160);
    datagram.kind = MediaDatagramKind::ScreenFrame;
    datagram.payload.resize(375_000, 0);

    assert_eq!(
        tracker.inspect_at(session_id, &datagram, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    datagram.sequence = 2;
    datagram.flags = 0;
    datagram.payload = vec![1];
    assert!(matches!(
        tracker.inspect_at(session_id, &datagram, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
    ));
}

fn video_datagram(sequence: u64, key_frame: bool, width: u32, height: u32) -> MediaDatagram {
    MediaDatagram {
        track_id: uuid::Uuid::nil(),
        audio_source: None,
        audio_channels: 0,
        kind: MediaDatagramKind::CameraFrame,
        codec: MediaCodec::Vp9,
        flags: if key_frame {
            MEDIA_DATAGRAM_FLAG_KEY_FRAME
        } else {
            0
        },
        sequence,
        timestamp_us: 0,
        duration_us: 0,
        room_id: Uuid::new_v4(),
        sender_user_id: Uuid::nil(),
        payload: if key_frame {
            vp9_key_frame(width, height)
        } else {
            Vec::new()
        },
    }
}

fn fragmented(mut datagram: MediaDatagram, index: u16, count: u16) -> MediaDatagram {
    let bytes = std::mem::take(&mut datagram.payload);
    datagram.flags |= MEDIA_DATAGRAM_FLAG_FRAGMENTED;
    datagram.payload = Vec::new();
    datagram
        .payload
        .extend_from_slice(&((bytes.len() + usize::from(count) - 1) as u32).to_be_bytes());
    datagram.payload.extend_from_slice(&index.to_be_bytes());
    datagram.payload.extend_from_slice(&count.to_be_bytes());
    if index == 0 {
        datagram.payload.extend_from_slice(&bytes);
    } else {
        datagram.payload.push(0);
    }
    datagram
}

fn vp9_key_frame(width: u32, height: u32) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.write(0b10, 2);
    writer.write(0, 1);
    writer.write(0, 1);
    writer.write(0, 1);
    writer.write(0, 1);
    writer.write(1, 1);
    writer.write(0, 1);
    writer.write(0x49_83_42, 24);
    writer.write(1, 3);
    writer.write(0, 1);
    writer.write(width - 1, 16);
    writer.write(height - 1, 16);
    writer.bytes
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bit_offset: usize,
}

impl BitWriter {
    fn write(&mut self, value: u32, count: usize) {
        for bit_index in (0..count).rev() {
            if self.bit_offset.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let bit = ((value >> bit_index) & 1) as u8;
            let byte_index = self.bit_offset / 8;
            let shift = 7 - self.bit_offset % 8;
            self.bytes[byte_index] |= bit << shift;
            self.bit_offset += 1;
        }
    }
}

#[test]
fn oversized_recovery_keyframe_borrows_budget_and_repayment_prevents_burst() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let mut key = video_datagram(1, true, 1920, 1080);
    key.kind = MediaDatagramKind::ScreenFrame;
    key.payload.resize(500_000, 0);

    assert_eq!(
        tracker.inspect_at(session_id, &key, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    key.sequence = 2;
    key.flags = 0;
    key.payload = vec![1];
    assert!(matches!(
        tracker.inspect_at(session_id, &key, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
    ));
    key.sequence = 3;
    key.flags = MEDIA_DATAGRAM_FLAG_KEY_FRAME;
    key.payload = vp9_key_frame(1920, 1080);
    assert_eq!(
        tracker.inspect_at(
            session_id,
            &key,
            SCREEN_SHARE_BITRATE_LIMIT,
            now + Duration::from_secs(1)
        ),
        VideoAdmission::Forward
    );
}

#[test]
fn recovery_keyframe_larger_than_bounded_credit_is_rejected_even_after_waiting() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let mut frame = video_datagram(1, true, 1920, 1080);
    frame.kind = MediaDatagramKind::ScreenFrame;
    frame.payload.resize(750_001, 0);

    for seconds in [0, 10, 30] {
        frame.sequence += 1;
        assert!(matches!(
            tracker.inspect_at(
                session_id,
                &frame,
                SCREEN_SHARE_BITRATE_LIMIT,
                now + Duration::from_secs(seconds)
            ),
            VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
        ));
    }
}

#[test]
fn fragmented_recovery_keyframe_charges_full_size_only_once() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let mut key = video_datagram(1, true, 1920, 1080);
    key.kind = MediaDatagramKind::ScreenFrame;
    key.payload.resize(500_000, 0);
    let first = fragmented(key.clone(), 0, 2);
    let second = fragmented(key, 1, 2);

    assert_eq!(
        tracker.inspect_at(session, &first, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &second, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(tracker.publications[0].budget_bits, Some(-1_000_008));
}

#[test]
fn repeated_recovery_keyframes_cannot_exceed_credit_and_recover_after_refill() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let mut frame = video_datagram(1, true, 1920, 1080);
    frame.kind = MediaDatagramKind::ScreenFrame;
    frame.payload.resize(500_000, 0);

    assert_eq!(
        tracker.inspect_at(session, &frame, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    for sequence in 2..5 {
        frame.sequence = sequence;
        assert!(matches!(
            tracker.inspect_at(session, &frame, SCREEN_SHARE_BITRATE_LIMIT, now),
            VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
        ));
    }
    frame.sequence = 5;
    assert_eq!(
        tracker.inspect_at(
            session,
            &frame,
            SCREEN_SHARE_BITRATE_LIMIT,
            now + Duration::from_secs(10)
        ),
        VideoAdmission::Forward
    );
}

#[test]
fn stopping_and_restarting_source_preserves_bitrate_debt() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let mut frame = video_datagram(1, true, 1920, 1080);
    frame.kind = MediaDatagramKind::ScreenFrame;
    frame.payload.resize(500_000, 0);
    assert_eq!(
        tracker.inspect_at(session, &frame, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );

    tracker.remove_source(session, frame.room_id, frame.kind);
    frame.sequence = 2;

    assert!(matches!(
        tracker.inspect_at(session, &frame, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
    ));
    assert_eq!(tracker.active_source_count(now), 0);
}

#[test]
fn replaying_accepted_first_fragment_does_not_forward_unaccounted_bytes() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let mut frame = video_datagram(1, true, 1920, 1080);
    frame.kind = MediaDatagramKind::ScreenFrame;
    let first = fragmented(frame, 0, 2);

    assert_eq!(
        tracker.inspect_at(session, &first, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &first, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::DuplicateFrame)
    );
}
