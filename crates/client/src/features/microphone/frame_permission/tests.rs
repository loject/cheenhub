use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn cloned_frame_permission_observes_capture_revocation() {
    let closed = Arc::new(AtomicBool::new(false));
    let check_closed = closed.clone();
    let permission = FramePermission(Arc::new(move || !check_closed.load(Ordering::Acquire)));
    let queued_permission = permission.clone();
    assert!(queued_permission.allowed());

    closed.store(true, Ordering::Release);

    assert!(!permission.allowed());
    assert!(!queued_permission.allowed());
}
