use super::*;

fn pieces() -> Vec<MediaDatagram> {
    let frame = video_datagram(1, true, 1280, 720);
    let mut first = fragmented(frame, 0, 3);
    let total = first.payload.len() - 8 + 2;
    first.payload[..4].copy_from_slice(&(total as u32).to_be_bytes());
    let mut middle = first.clone();
    middle.payload.truncate(8);
    middle.payload[4..6].copy_from_slice(&1_u16.to_be_bytes());
    middle.payload.push(7);
    let mut last = middle.clone();
    last.payload[4..6].copy_from_slice(&2_u16.to_be_bytes());
    vec![first, middle, last]
}

#[test]
fn repeated_middle_fragment_is_not_forwarded() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();

    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[1], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[1], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::DuplicateFrame)
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_ne!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
}

#[test]
fn changed_fragment_count_invalidates_remaining_fragments() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();
    let mut changed = frames[1].clone();
    changed.payload[6..8].copy_from_slice(&4_u16.to_be_bytes());

    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &changed, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
    assert_ne!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
}

#[test]
fn completed_frame_outside_recent_window_cannot_be_replayed() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let mut frame = video_datagram(1, true, 1280, 720);
    let first = frame.clone();
    for sequence in 1..=130 {
        frame.sequence = sequence;
        assert_eq!(
            tracker.inspect_at(session, &frame, CAMERA_BITRATE_LIMIT, now),
            VideoAdmission::Forward
        );
    }

    assert_ne!(
        tracker.inspect_at(session, &first, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
}

#[test]
fn fragments_after_first_can_arrive_out_of_order() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();

    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[1], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
}

#[test]
fn inconsistent_first_fragment_poisoned_frame_cannot_continue() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();
    let mut changed = frames[0].clone();
    changed.payload[6..8].copy_from_slice(&4_u16.to_be_bytes());

    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &changed, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[1], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
}

#[test]
fn malformed_header_prevents_remaining_fragments_from_being_forwarded() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();
    let mut damaged = frames[1].clone();
    damaged.payload.truncate(3);

    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &damaged, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
}

#[test]
fn first_fragment_is_required_but_remaining_fragments_can_reorder() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();

    assert_eq!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::AwaitingFirstFragment)
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[1], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
}

#[test]
fn duplicate_fragments_do_not_consume_extra_budget() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();
    let size = u32::from_be_bytes(frames[0].payload[..4].try_into().unwrap()) as usize;
    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &frames[1], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    for _ in 0..5 {
        assert_eq!(
            tracker.inspect_at(session, &frames[1], CAMERA_BITRATE_LIMIT, now),
            VideoAdmission::Drop(VideoDropReason::DuplicateFrame)
        );
    }
    assert_eq!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    let mut follow = video_datagram(2, false, 1280, 720);
    follow.room_id = frames[0].room_id;
    follow.payload.resize(175_000 - size, 0);

    assert_eq!(
        tracker.inspect_at(session, &follow, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
}

#[test]
fn empty_fragment_is_rejected_and_poisoned_frame_is_not_forwarded() {
    let mut tracker = VideoPublicationTracker::default();
    let session = Uuid::new_v4();
    let now = Instant::now();
    let frames = pieces();
    let mut empty = frames[1].clone();
    empty.payload.truncate(8);

    assert_eq!(
        tracker.inspect_at(session, &frames[0], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
    assert_eq!(
        tracker.inspect_at(session, &empty, CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Drop(VideoDropReason::MalformedFragment)
    );
    assert_ne!(
        tracker.inspect_at(session, &frames[2], CAMERA_BITRATE_LIMIT, now),
        VideoAdmission::Forward
    );
}
