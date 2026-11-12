use super::*;

#[test]
fn overload_drops_frames_and_preserves_pending_keyframe() {
    let mut state = EncoderLoad::default();

    assert!(!state.accept(2, true));
    assert_eq!(state.dropped, 1);
    assert!(state.accept(0, false));
    assert!(state.take_key_frame(false));
    assert!(!state.take_key_frame(false));
}

#[test]
fn output_latency_matches_submitted_timestamp_and_stays_bounded() {
    let mut state = EncoderLoad::default();
    for timestamp in 0..100 {
        state.submitted(timestamp, timestamp as f64);
    }

    assert_eq!(state.output_latency(99, 110.0), Some(11.0));
    assert_eq!(state.output_latency(0, 110.0), None);
}

#[test]
fn reconfiguration_discards_unmatched_old_timings() {
    let mut state = EncoderLoad::default();
    state.submitted(7, 10.0);

    state.clear_timings();
    state.submitted(7, 30.0);

    assert_eq!(state.output_latency(7, 50.0), Some(20.0));
}

#[test]
fn repeated_timestamps_use_submission_order() {
    let mut state = EncoderLoad::default();
    state.submitted(7, 10.0);
    state.submitted(7, 20.0);

    assert_eq!(state.output_latency(7, 30.0), Some(20.0));
    assert_eq!(state.output_latency(7, 40.0), Some(20.0));
}
