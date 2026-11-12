use super::claim_stop;
use std::cell::Cell;

#[test]
fn simultaneous_manual_and_chrome_stop_claim_cleanup_once() {
    let stopped = Cell::new(false);
    let cleanup_calls = Cell::new(0);

    for _reason in ["manual", "chrome"] {
        if claim_stop(&stopped) {
            cleanup_calls.set(cleanup_calls.get() + 1);
        }
    }

    assert!(stopped.get());
    assert_eq!(cleanup_calls.get(), 1);
}
