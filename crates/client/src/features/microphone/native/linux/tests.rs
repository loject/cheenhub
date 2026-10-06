use super::*;

#[test]
fn dropping_startup_guard_requests_worker_shutdown() {
    let closed = Arc::new(AtomicBool::new(false));
    drop(Cancellation(closed.clone()));
    assert!(closed.load(Ordering::Relaxed));
}
