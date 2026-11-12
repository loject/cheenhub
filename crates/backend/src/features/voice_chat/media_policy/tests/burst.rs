use super::*;

#[test]
fn first_keyframe_can_borrow_and_camera_and_screen_budgets_are_independent() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let mut camera = video_datagram(1, true, 1280, 720);
    camera.payload.resize(200_000, 0);
    let mut screen = camera.clone();
    screen.kind = MediaDatagramKind::ScreenFrame;
    screen.payload.resize(500_000, 0);

    assert_eq!(
        tracker.inspect_at(session, &camera, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &screen, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    camera.sequence = 2;
    camera.flags = 0;
    camera.payload = vec![1];
    screen.sequence = 2;
    screen.flags = 0;
    screen.payload = vec![1];
    assert!(matches!(
        tracker.inspect_at(session, &camera, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
    ));
    assert!(matches!(
        tracker.inspect_at(session, &screen, SCREEN_SHARE_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::BitrateLimitExceeded { .. })
    ));
}
