use super::*;

#[test]
fn countdown_does_not_advance_while_paused() {
    let mut remaining_ms = TOAST_TTL_MS;

    assert!(!advance_remaining(&mut remaining_ms, 1_000, true));
    assert_eq!(remaining_ms, TOAST_TTL_MS);
}

#[test]
fn countdown_continues_from_remaining_time_after_pause() {
    let mut remaining_ms = 1_000;

    assert!(!advance_remaining(&mut remaining_ms, 400, false));
    assert!(!advance_remaining(&mut remaining_ms, 500, true));
    assert!(!advance_remaining(&mut remaining_ms, 500, false));
    assert!(advance_remaining(&mut remaining_ms, 100, false));
}
