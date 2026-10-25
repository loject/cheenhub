//! Регрессии допуска демонстрации окон с произвольными пропорциями.

use super::*;

#[test]
fn screen_policy_accepts_window_dimensions_within_allowed_presets() {
    for (width, height) in [
        (1280, 357),
        (1280, 386),
        (1280, 450),
        (1920, 540),
        (405, 720),
        (640, 480),
    ] {
        let mut tracker = VideoPublicationTracker::default();
        let mut datagram = video_datagram(0, true, width, height);
        datagram.kind = MediaDatagramKind::ScreenFrame;

        let admission = tracker.inspect_at(
            Uuid::new_v4(),
            &datagram,
            BASE_SCREEN_SHARE_VIDEO_PRESETS,
            Instant::now(),
        );

        assert_eq!(admission, VideoAdmission::Forward, "{width}x{height}");
    }
}

#[test]
fn screen_policy_rejects_dimensions_exceeding_either_preset_bound() {
    for (width, height) in [(1921, 357), (405, 1081), (2560, 714)] {
        let mut tracker = VideoPublicationTracker::default();
        let mut datagram = video_datagram(0, true, width, height);
        datagram.kind = MediaDatagramKind::ScreenFrame;

        let admission = tracker.inspect_at(
            Uuid::new_v4(),
            &datagram,
            BASE_SCREEN_SHARE_VIDEO_PRESETS,
            Instant::now(),
        );

        assert_eq!(
            admission,
            VideoAdmission::Drop(VideoDropReason::UnsupportedResolution { width, height }),
        );
    }
}

#[test]
fn screen_policy_does_not_use_camera_permissions_for_window_dimensions() {
    let mut tracker = VideoPublicationTracker::default();
    let mut datagram = video_datagram(0, true, 1280, 357);
    datagram.kind = MediaDatagramKind::ScreenFrame;

    let admission = tracker.inspect_at(
        Uuid::new_v4(),
        &datagram,
        BASE_CAMERA_VIDEO_PRESETS,
        Instant::now(),
    );

    assert!(matches!(admission, VideoAdmission::Drop(_)));
}

#[test]
fn camera_policy_keeps_exact_preset_dimensions() {
    let mut tracker = VideoPublicationTracker::default();
    let datagram = video_datagram(0, true, 1280, 357);

    let admission = tracker.inspect_at(
        Uuid::new_v4(),
        &datagram,
        BASE_CAMERA_VIDEO_PRESETS,
        Instant::now(),
    );

    assert_eq!(
        admission,
        VideoAdmission::Drop(VideoDropReason::UnsupportedResolution {
            width: 1280,
            height: 357,
        }),
    );
}

#[test]
fn screen_policy_forwards_all_fragments_of_nonstandard_key_frame() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let now = Instant::now();
    let mut datagram = video_datagram(0, true, 1280, 357);
    datagram.kind = MediaDatagramKind::ScreenFrame;
    let first = fragmented(datagram.clone(), 0, 2);
    let second = fragmented(datagram, 1, 2);

    for fragment in [first, second] {
        assert_eq!(
            tracker.inspect_at(session_id, &fragment, BASE_SCREEN_SHARE_VIDEO_PRESETS, now),
            VideoAdmission::Forward,
        );
    }
}

#[test]
fn screen_policy_allows_30_fps_for_small_window_regardless_of_preset_order() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let started = Instant::now();
    let mut datagram = video_datagram(0, true, 1280, 357);
    datagram.kind = MediaDatagramKind::ScreenFrame;
    let presets = [VideoPresetId::Screen1080p15, VideoPresetId::Screen720p30];

    assert_eq!(
        tracker.inspect_at(session_id, &datagram, &presets, started),
        VideoAdmission::Forward
    );
    datagram.flags = 0;
    datagram.payload.clear();
    for sequence in 1..=30 {
        datagram.sequence = sequence;
        let admission = tracker.inspect_at(
            session_id,
            &datagram,
            &presets,
            started + Duration::from_millis(sequence * 34),
        );

        assert_eq!(admission, VideoAdmission::Forward, "frame {sequence}");
    }
}

#[test]
fn screen_policy_keeps_15_fps_limit_for_window_exceeding_720p_width() {
    let mut tracker = VideoPublicationTracker::default();
    let session_id = Uuid::new_v4();
    let started = Instant::now();
    let mut datagram = video_datagram(0, true, 1920, 540);
    datagram.kind = MediaDatagramKind::ScreenFrame;

    assert_eq!(
        tracker.inspect_at(
            session_id,
            &datagram,
            BASE_SCREEN_SHARE_VIDEO_PRESETS,
            started
        ),
        VideoAdmission::Forward
    );
    datagram.flags = 0;
    datagram.payload.clear();
    for sequence in 1..=29 {
        datagram.sequence = sequence;
        assert_eq!(
            tracker.inspect_at(
                session_id,
                &datagram,
                BASE_SCREEN_SHARE_VIDEO_PRESETS,
                started + Duration::from_millis(sequence * 34),
            ),
            VideoAdmission::Forward
        );
    }
    datagram.sequence = 30;

    let admission = tracker.inspect_at(
        session_id,
        &datagram,
        BASE_SCREEN_SHARE_VIDEO_PRESETS,
        started + Duration::from_millis(1020),
    );

    assert!(matches!(
        admission,
        VideoAdmission::Drop(VideoDropReason::FpsLimitExceeded { max_fps: 15, .. })
    ));
}
