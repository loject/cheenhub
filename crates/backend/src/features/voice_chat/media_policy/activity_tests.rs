//! Проверки подсчёта активных видеоисточников голосового чата.

use cheenhub_contracts::media::MediaDatagramKind;
use cheenhub_contracts::video_presets::{
    BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS,
};
use uuid::Uuid;

use super::test_support::{key_frame_datagram, presence, screen_datagram, sized_key_frame};
use super::{VideoAdmission, VideoDropReason, VideoPublicationTracker};
use std::time::{Duration, Instant};

#[test]
fn camera_and_screen_of_one_session_count_as_two_sources() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let now = Instant::now();
    let camera = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);
    let screen = screen_datagram(room_id);

    assert_eq!(
        tracker.inspect_at(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session_id, &screen, BASE_SCREEN_SHARE_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );
    assert_eq!(tracker.active_source_count(now), 2);
}

#[test]
fn two_sessions_of_one_user_are_not_merged() {
    let mut tracker = VideoPublicationTracker::default();
    let room_id = Uuid::new_v4();
    let now = Instant::now();
    let camera = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);
    let screen = screen_datagram(room_id);

    for session_id in [Uuid::new_v4(), Uuid::new_v4()] {
        assert_eq!(
            tracker.inspect_at(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS, now),
            VideoAdmission::Forward
        );
        assert_eq!(
            tracker.inspect_at(session_id, &screen, BASE_SCREEN_SHARE_VIDEO_PRESETS, now),
            VideoAdmission::Forward
        );
    }

    assert_eq!(tracker.active_source_count(now), 4);
}

#[test]
fn repeated_packets_do_not_add_sources_or_extend_activity() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let started = Instant::now();
    let camera = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);

    assert_eq!(
        tracker.inspect_at(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS, started),
        VideoAdmission::Forward
    );
    // Повторы с той же последовательностью возвращают кэшированное решение и не
    // продлевают активность: после окна в 10 секунд источник перестаёт считаться активным.
    for repeat in 1..=5u64 {
        let moment = started + Duration::from_secs(repeat * 5);
        assert_eq!(
            tracker.inspect_at(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS, moment),
            VideoAdmission::Forward
        );
        assert_eq!(tracker.publications.len(), 1);
        let expected = u32::from(repeat < 2);
        assert_eq!(
            tracker.active_source_count(moment) as u32,
            expected,
            "повтор пакета с той же последовательностью не продлевает активность"
        );
    }
}

#[test]
fn source_expires_after_ten_seconds_without_frames() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let started = Instant::now();
    let camera = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);

    assert_eq!(
        tracker.inspect_at(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS, started),
        VideoAdmission::Forward
    );

    assert_eq!(
        tracker.active_source_count(started + Duration::from_secs(9)),
        1
    );
    assert_eq!(
        tracker.active_source_count(started + Duration::from_secs(10)),
        0
    );
}

#[test]
fn rejected_video_never_becomes_active() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let started = Instant::now();
    let oversized = sized_key_frame(MediaDatagramKind::CameraFrame, room_id, 1920, 1080);
    let mut awaiting_key_frame = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);
    awaiting_key_frame.sequence = 2;
    awaiting_key_frame.flags = 0;
    awaiting_key_frame.payload.clear();

    assert!(matches!(
        tracker.inspect_at(session_id, &oversized, BASE_CAMERA_VIDEO_PRESETS, started),
        VideoAdmission::Drop(VideoDropReason::UnsupportedResolution { .. })
    ));
    assert_eq!(
        tracker.inspect_at(
            session_id,
            &awaiting_key_frame,
            BASE_CAMERA_VIDEO_PRESETS,
            started
        ),
        VideoAdmission::Drop(VideoDropReason::AwaitingKeyFrame)
    );
    assert_eq!(tracker.active_source_count(started), 0);
}

#[test]
fn stopping_camera_keeps_screen_active() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let now = Instant::now();
    let camera = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);
    let screen = screen_datagram(room_id);

    assert_eq!(
        tracker.inspect_at(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session_id, &screen, BASE_SCREEN_SHARE_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );

    tracker.remove_source(session_id, room_id, MediaDatagramKind::CameraFrame);

    assert_eq!(tracker.active_source_count(now), 1);
}

#[test]
fn leaving_presence_removes_all_its_sources() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let now = Instant::now();
    let camera = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);
    let screen = screen_datagram(room_id);

    assert_eq!(
        tracker.inspect_at(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session_id, &screen, BASE_SCREEN_SHARE_VIDEO_PRESETS, now),
        VideoAdmission::Forward
    );

    tracker.remove_presences(&[presence(session_id, room_id)]);

    assert_eq!(tracker.active_source_count(now), 0);
}
