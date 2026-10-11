use super::*;

#[test]
fn stale_failure_cannot_disable_reenabled_dsp() {
    let runtime = RuntimeSwitch::new(true);
    let old = runtime.snapshot();
    runtime.store(true, Ordering::Relaxed);

    assert!(!runtime.fail(old));
    assert!(runtime.load(Ordering::Relaxed));
    assert!(!runtime.failed());
}

#[test]
fn failure_is_visible_and_retry_clears_it() {
    let runtime = RuntimeSwitch::new(true);

    assert!(runtime.fail(runtime.snapshot()));
    assert!(!runtime.load(Ordering::Relaxed));
    assert!(runtime.failed());
    runtime.store(true, Ordering::Relaxed);
    assert!(runtime.load(Ordering::Relaxed));
    assert!(!runtime.failed());
}

#[test]
fn rollback_preserves_failure_state_with_a_fresh_revision() {
    let runtime = RuntimeSwitch::new(true);
    runtime.fail(runtime.snapshot());
    let previous = runtime.snapshot();
    runtime.store(true, Ordering::Relaxed);
    let rejected = runtime.snapshot();

    runtime.restore(previous);

    assert!(runtime.failed());
    assert!(!runtime.load(Ordering::Relaxed));
    assert_ne!(runtime.snapshot(), previous);
    assert!(!runtime.fail(rejected));
}
