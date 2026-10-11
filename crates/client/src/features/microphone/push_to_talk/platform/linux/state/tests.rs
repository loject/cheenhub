use super::*;

#[test]
fn recovery_requires_a_new_activation() {
    let state = State::default();
    let recording = AtomicU64::new(0);
    state.recover();
    assert_eq!(state.held_epoch(&recording), None);

    state.event(true);

    assert!(state.held_epoch(&recording).is_some());
}

#[test]
fn failure_revokes_audio_and_ignores_events_until_recovery() {
    let state = State::default();
    let recording = AtomicU64::new(0);
    state.recover();
    state.event(true);
    let previous = state.held_epoch(&recording).unwrap();

    state.fail();
    state.event(true);

    assert_eq!(state.held_epoch(&recording), None);
    state.recover();
    assert_eq!(state.held_epoch(&recording), None);
    state.event(true);
    assert_ne!(state.held_epoch(&recording).unwrap(), previous);
}

#[test]
fn release_and_repress_never_reuse_an_epoch() {
    let state = State::default();
    let recording = AtomicU64::new(0);
    state.recover();
    state.event(true);
    let previous = state.held_epoch(&recording).unwrap();

    state.event(false);
    assert_eq!(state.held_epoch(&recording), None);
    state.event(true);

    assert_ne!(state.held_epoch(&recording).unwrap(), previous);
}

#[test]
fn recording_pause_invalidates_captured_audio() {
    let state = State::default();
    let recording = AtomicU64::new(0);
    state.recover();
    state.event(true);
    let previous = state.held_epoch(&recording).unwrap();

    recording.store(1, Ordering::Release);
    assert_eq!(state.held_epoch(&recording), None);
    recording.store(2, Ordering::Release);

    assert_ne!(state.held_epoch(&recording).unwrap(), previous);
}

#[test]
fn stalled_worker_cannot_leave_microphone_open() {
    let state = State::default();
    let recording = AtomicU64::new(0);
    state.recover();
    state.event(true);
    state.inner.lock().unwrap().heartbeat = Some(Instant::now() - Duration::from_secs(2));

    assert_eq!(state.held_epoch(&recording), None);
}

#[test]
fn watchdog_recovery_requires_a_new_press() {
    let state = State::default();
    let recording = AtomicU64::new(0);
    state.recover();
    state.event(true);
    let previous = state.held_epoch(&recording).unwrap();
    state.inner.lock().unwrap().heartbeat = Some(Instant::now() - Duration::from_secs(2));

    state.heartbeat();

    assert_eq!(state.held_epoch(&recording), None);
    state.event(true);
    assert_ne!(state.held_epoch(&recording).unwrap(), previous);
}
