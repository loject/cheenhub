use super::{ActivityGate, ActivityPolicy, BackgroundActivity};
use futures_channel::mpsc;
use futures_util::{FutureExt, StreamExt};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn suspends_hidden_application_without_active_session() {
    let policy = ActivityPolicy {
        visible: false,
        background_required: false,
    };
    assert!(!policy.allows_connection());
}

#[test]
fn preserves_hidden_active_session_including_muted_call() {
    let mut policy = ActivityPolicy {
        visible: false,
        background_required: true,
    };
    assert!(policy.allows_connection());
    policy.background_required = false;
    assert!(!policy.allows_connection());
}

#[test]
fn resumes_visible_application_without_active_session() {
    let mut policy = ActivityPolicy::default();
    assert!(!policy.allows_connection());
    policy.visible = true;
    assert!(policy.allows_connection());
}

#[test]
fn hidden_runtime_waits_without_polling_until_visible() {
    let (visibility, events) = mpsc::unbounded();
    visibility.unbounded_send(false).unwrap();
    let activity = BackgroundActivity::default();
    let mut gate = ActivityGate::new(events.boxed_local(), activity.subscribe());
    let mut wait = Box::pin(gate.wait_until_allowed());
    assert!(wait.as_mut().now_or_never().is_none());
    visibility.unbounded_send(true).unwrap();
    assert_eq!(wait.as_mut().now_or_never(), Some(()));
}

#[test]
fn suspension_cancels_inflight_connection_runtime() {
    struct Cancellation(Rc<Cell<bool>>);
    impl Drop for Cancellation {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let (visibility, events) = mpsc::unbounded();
    visibility.unbounded_send(true).unwrap();
    let activity = BackgroundActivity::default();
    let mut gate = ActivityGate::new(events.boxed_local(), activity.subscribe());
    assert_eq!(gate.wait_until_allowed().now_or_never(), Some(()));
    let cancelled = Rc::new(Cell::new(false));
    let guard = Cancellation(cancelled.clone());
    let operation = async move {
        let _guard = guard;
        std::future::pending::<()>().await;
    };
    let mut running = Box::pin(gate.run_until_suspended(operation));
    assert!(running.as_mut().now_or_never().is_none());
    visibility.unbounded_send(false).unwrap();
    assert_eq!(running.as_mut().now_or_never(), Some(None));
    assert!(cancelled.get());
}

#[test]
fn background_session_keeps_runtime_until_call_ends() {
    let (visibility, events) = mpsc::unbounded();
    visibility.unbounded_send(false).unwrap();
    let activity = BackgroundActivity::default();
    activity.set_required(true);
    let mut gate = ActivityGate::new(events.boxed_local(), activity.subscribe());
    assert_eq!(gate.wait_until_allowed().now_or_never(), Some(()));
    let mut running = Box::pin(gate.run_until_suspended(std::future::pending::<()>()));
    assert!(running.as_mut().now_or_never().is_none());
    activity.set_required(false);
    assert_eq!(running.as_mut().now_or_never(), Some(None));
}

#[test]
fn rapid_hide_show_uses_latest_visibility_and_does_not_cancel_operation() {
    let (visibility, events) = mpsc::unbounded();
    visibility.unbounded_send(true).unwrap();
    let activity = BackgroundActivity::default();
    let mut gate = ActivityGate::new(events.boxed_local(), activity.subscribe());
    assert_eq!(gate.wait_until_allowed().now_or_never(), Some(()));
    visibility.unbounded_send(false).unwrap();
    visibility.unbounded_send(true).unwrap();
    assert_eq!(
        gate.run_until_suspended(async { 42 }).now_or_never(),
        Some(Some(42))
    );
}
