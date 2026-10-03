//! Проверки политики видеопубликаций и подсчёта активных видеоисточников.

mod activity;
pub(crate) mod support;

use super::*;
use cheenhub_contracts::{
    media::{MediaCodec, MediaDatagram},
    video_presets::{BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS},
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
                BASE_SCREEN_SHARE_VIDEO_PRESETS,
                Instant::now(),
            ),
            VideoAdmission::Forward
        );
    }
}

#[test]
fn camera_policy_rejects_1080p() {
    let mut tracker = VideoPublicationTracker::default();
    let datagram = video_datagram(1, true, 1920, 1080);
    assert_eq!(
        tracker.inspect_at(
            Uuid::new_v4(),
            &datagram,
            BASE_CAMERA_VIDEO_PRESETS,
            Instant::now(),
        ),
        VideoAdmission::Drop(VideoDropReason::UnsupportedResolution {
            width: 1920,
            height: 1080,
        })
    );
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
        tracker.inspect_at(session_id, &first, BASE_CAMERA_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session_id, &second, BASE_CAMERA_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );
    assert_eq!(tracker.publications[0].window_frames, 1);
}

#[test]
fn sustained_fps_violation_blocks_until_later_key_frame() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let started = Instant::now();
    let key = video_datagram(1, true, 1280, 720);
    let room_id = key.room_id;
    assert_eq!(
        tracker.inspect_at(session_id, &key, BASE_CAMERA_VIDEO_PRESETS, started),
        VideoAdmission::Forward
    );
    for sequence in 2..=26 {
        let mut frame = video_datagram(sequence, false, 0, 0);
        frame.room_id = room_id;
        assert_eq!(
            tracker.inspect_at(
                session_id,
                &frame,
                BASE_CAMERA_VIDEO_PRESETS,
                started + Duration::from_millis(sequence * 30),
            ),
            VideoAdmission::Forward
        );
    }
    let mut violating = video_datagram(27, false, 0, 0);
    violating.room_id = room_id;
    assert!(matches!(
        tracker.inspect_at(
            session_id,
            &violating,
            BASE_CAMERA_VIDEO_PRESETS,
            started + Duration::from_millis(1_010),
        ),
        VideoAdmission::Drop(VideoDropReason::FpsLimitExceeded { .. })
    ));
    let mut early_key = video_datagram(28, true, 1280, 720);
    early_key.room_id = room_id;
    assert_eq!(
        tracker.inspect_at(
            session_id,
            &early_key,
            BASE_CAMERA_VIDEO_PRESETS,
            started + Duration::from_millis(1_500),
        ),
        VideoAdmission::Drop(VideoDropReason::FpsBlockActive)
    );
    let mut later_key = video_datagram(29, true, 1280, 720);
    later_key.room_id = room_id;
    assert_eq!(
        tracker.inspect_at(
            session_id,
            &later_key,
            BASE_CAMERA_VIDEO_PRESETS,
            started + Duration::from_millis(2_100),
        ),
        VideoAdmission::Forward
    );
}

fn video_datagram(sequence: u64, key_frame: bool, width: u32, height: u32) -> MediaDatagram {
    MediaDatagram {
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
        .extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    datagram.payload.extend_from_slice(&index.to_be_bytes());
    datagram.payload.extend_from_slice(&count.to_be_bytes());
    datagram.payload.extend_from_slice(&bytes);
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
