//! Проверки жизненного цикла toast входящего личного звонка.

use super::*;

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
