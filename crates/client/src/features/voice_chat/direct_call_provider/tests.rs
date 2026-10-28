//! Проверки жизненного цикла toast входящего личного звонка.

use super::*;

#[test]
fn backgrounding_active_call_shows_system_notification_once() {
    assert!(should_show_incoming_call_notification(
        Some("call-1"),
        Some("call-1"),
        true,
        false,
        false,
    ));
    assert!(!should_show_incoming_call_notification(
        Some("call-1"),
        Some("call-1"),
        false,
        false,
        false,
    ));
    assert!(!should_show_incoming_call_notification(
        Some("call-1"),
        Some("call-1"),
        true,
        false,
        true,
    ));
}

#[test]
fn incoming_call_ring_starts_once_and_stops_when_call_ends() {
    let mut ring = IncomingCallRingState::default();

    let started = ring.sync(Some("call-1"), true, false);
    let repeated = ring.sync(Some("call-1"), true, false);
    let ended = ring.sync(None, true, false);

    assert_eq!(started.start.as_deref(), Some("call-1"));
    assert!(!started.stop);
    assert_eq!(repeated.start, None);
    assert!(!repeated.stop);
    assert_eq!(ended.start, None);
    assert!(ended.stop);
}

#[test]
fn ignored_incoming_call_ring_stays_stopped_after_recovery() {
    let mut ring = IncomingCallRingState::default();
    ring.sync(Some("call-1"), true, false);

    let ignored = ring.sync(Some("call-1"), true, true);
    let recovered = ring.sync(Some("call-1"), true, true);
    let next = ring.sync(Some("call-2"), true, false);

    assert!(ignored.stop);
    assert_eq!(ignored.start, None);
    assert!(!recovered.stop);
    assert_eq!(recovered.start, None);
    assert_eq!(next.start.as_deref(), Some("call-2"));
}

#[test]
fn incoming_call_ring_pauses_and_resumes_with_activity_visibility() {
    let mut ring = IncomingCallRingState::default();
    ring.sync(Some("call-1"), true, false);

    let backgrounded = ring.sync(Some("call-1"), false, false);
    let still_backgrounded = ring.sync(Some("call-1"), false, false);
    let foregrounded = ring.sync(Some("call-1"), true, false);

    assert!(backgrounded.stop);
    assert_eq!(backgrounded.start, None);
    assert!(!still_backgrounded.stop);
    assert_eq!(foregrounded.start.as_deref(), Some("call-1"));
}

#[test]
fn incoming_call_ring_switches_to_next_call_without_overlap() {
    let mut ring = IncomingCallRingState::default();
    ring.sync(Some("call-1"), true, false);

    let next = ring.sync(Some("call-2"), true, false);

    assert!(next.stop);
    assert_eq!(next.start.as_deref(), Some("call-2"));
}

#[test]
fn repeated_ringing_event_does_not_show_second_toast() {
    let mut state = IncomingCallToastState::default();

    let first = state.sync(Some("call-1"));
    let repeated = state.sync(Some("call-1"));

    assert_eq!(first.dismiss, None);
    assert!(first.show);
    assert_eq!(repeated.dismiss, None);
    assert!(!repeated.show);
}

#[test]
fn ignored_call_stays_hidden_after_connection_recovery() {
    let mut state = IncomingCallToastState::default();
    state.sync(Some("call-1"));

    assert!(state.ignore("call-1"));
    let ended = state.sync(None);
    let recovered = state.sync(Some("call-1"));

    assert_eq!(ended.dismiss.as_deref(), Some("incoming:call-1"));
    assert_eq!(recovered.dismiss, None);
    assert!(!recovered.show);
}

#[test]
fn next_call_shows_after_previously_ignored_call() {
    let mut state = IncomingCallToastState::default();
    state.sync(Some("call-1"));
    state.ignore("call-1");

    let next = state.sync(Some("call-2"));

    assert_eq!(next.dismiss.as_deref(), Some("incoming:call-1"));
    assert!(next.show);
}

#[test]
fn ignoring_stale_call_does_not_hide_current_call() {
    let mut state = IncomingCallToastState::default();
    state.sync(Some("call-1"));
    state.sync(Some("call-2"));

    let ignored = state.ignore("call-1");
    let repeated = state.sync(Some("call-2"));

    assert!(!ignored);
    assert!(!repeated.show);
    assert_eq!(state.ignored_call_id, None);
}

#[test]
fn active_call_can_refresh_toast_but_ignored_call_cannot_return() {
    let mut state = IncomingCallToastState::default();
    assert!(state.sync(Some("call-1")).show);
    assert!(state.should_display("call-1"));

    assert!(!state.sync(Some("call-1")).show);
    assert!(state.should_display("call-1"));

    assert!(state.ignore("call-1"));
    assert!(!state.should_display("call-1"));
    assert!(!state.should_display("call-2"));
}
