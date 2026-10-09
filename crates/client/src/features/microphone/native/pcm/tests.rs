use super::*;

#[test]
fn queued_pcm_from_previous_hold_is_rejected_after_repress() {
    assert!(!accepts_epoch(Some((1, 0)), Some((3, 0)), true));
    assert!(accepts_epoch(Some((3, 0)), Some((3, 0)), true));
}

#[test]
fn closed_gate_rejects_pcm_even_when_both_epochs_are_absent() {
    assert!(!accepts_epoch(None, None, true));
    assert!(!accepts_epoch(Some((1, 0)), None, true));
    assert!(accepts_epoch(None, None, false));
}

#[test]
fn first_device_buffer_after_press_is_discarded() {
    let mut previous = None;

    assert!(!accepts_capture_epoch(&mut previous, Some((1, 0)), true));
    assert!(accepts_capture_epoch(&mut previous, Some((1, 0)), true));
    assert!(!accepts_capture_epoch(&mut previous, None, true));
    assert!(!accepts_capture_epoch(&mut previous, Some((3, 0)), true));
    assert!(accepts_capture_epoch(&mut previous, Some((3, 0)), true));
}
