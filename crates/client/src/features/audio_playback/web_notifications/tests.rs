use super::{IncomingCallRingState, connection_loop_start_time};

#[test]
fn cancelled_incoming_call_ring_rejects_pending_audio_load() {
    let mut state = IncomingCallRingState::default();

    let pending_generation = state.begin();
    assert!(state.is_current(pending_generation));
    let _ = state.cancel();

    assert!(!state.is_current(pending_generation));
}

#[test]
fn loop_starts_after_lost_sample_finishes() {
    assert_eq!(connection_loop_start_time(10.0, 12.5), 12.5);
}

#[test]
fn loop_starts_immediately_when_lost_sample_has_already_finished() {
    assert_eq!(connection_loop_start_time(13.0, 12.5), 13.0);
}
