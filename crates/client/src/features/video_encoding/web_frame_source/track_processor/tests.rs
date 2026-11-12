use super::{ReadDisposition, mark_cancelled, release_lock_once, route_read_result};
use std::cell::Cell;

#[test]
fn cancelled_reader_closes_received_frame_once() {
    let closed = Cell::new(0);

    let result = route_read_result(true, false, Some(()), |_| closed.set(closed.get() + 1));

    assert!(matches!(result, ReadDisposition::Done));
    assert_eq!(closed.get(), 1);
}

#[test]
fn cancelled_reader_without_frame_does_not_close_a_value() {
    let closed = Cell::new(0);

    let result = route_read_result::<()>(true, false, None, |_| closed.set(closed.get() + 1));

    assert!(matches!(result, ReadDisposition::Done));
    assert_eq!(closed.get(), 0);
}

#[test]
fn completed_stream_does_not_close_a_value() {
    let closed = Cell::new(0);

    let result = route_read_result(true, true, Some(()), |_| closed.set(closed.get() + 1));

    assert!(matches!(result, ReadDisposition::Done));
    assert_eq!(closed.get(), 0);
}

#[test]
fn live_reader_transfers_frame_without_closing_it() {
    let closed = Cell::new(0);

    let result = route_read_result(false, false, Some(7), |_| closed.set(closed.get() + 1));

    assert!(matches!(result, ReadDisposition::Frame(7)));
    assert_eq!(closed.get(), 0);
}

#[test]
fn release_lock_marks_reader_released_only_after_success() {
    let attempted = Cell::new(false);
    let released = Cell::new(false);
    let calls = Cell::new(0);

    let result = release_lock_once(false, true, &attempted, &released, || {
        calls.set(calls.get() + 1);
        Ok::<(), ()>(())
    });

    assert_eq!(result, Ok(true));
    assert!(attempted.get());
    assert!(released.get());
    assert_eq!(calls.get(), 1);
}

#[test]
fn release_lock_failure_is_reported_without_retry() {
    let attempted = Cell::new(false);
    let released = Cell::new(false);
    let calls = Cell::new(0);

    let result = release_lock_once(false, true, &attempted, &released, || {
        calls.set(calls.get() + 1);
        Err("releaseLock failed")
    });
    let second_result = release_lock_once(false, true, &attempted, &released, || {
        calls.set(calls.get() + 1);
        Ok::<(), &str>(())
    });

    assert_eq!(result, Err("releaseLock failed"));
    assert_eq!(second_result, Ok(false));
    assert!(attempted.get());
    assert!(!released.get());
    assert_eq!(calls.get(), 1);
}

#[test]
fn release_lock_waits_for_pending_read_then_releases_once() {
    let attempted = Cell::new(false);
    let released = Cell::new(false);
    let calls = Cell::new(0);

    let pending = release_lock_once(true, true, &attempted, &released, || {
        calls.set(calls.get() + 1);
        Ok::<(), ()>(())
    });
    let finished = release_lock_once(false, true, &attempted, &released, || {
        calls.set(calls.get() + 1);
        Ok::<(), ()>(())
    });

    assert_eq!(pending, Ok(false));
    assert_eq!(finished, Ok(true));
    assert!(released.get());
    assert_eq!(calls.get(), 1);
}

#[test]
fn repeated_reader_stop_marks_cancellation_once() {
    let cancelled = Cell::new(false);

    assert!(mark_cancelled(&cancelled));
    assert!(!mark_cancelled(&cancelled));
    assert!(cancelled.get());
}
