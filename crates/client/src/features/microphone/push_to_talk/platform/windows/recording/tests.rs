use super::*;

#[test]
fn binding_completes_on_release_not_on_press_or_repeat() {
    let mut recording = Recording::default();
    let key = PushToTalkKey::from_value("163");

    assert_eq!(recording.input(key, true), None);
    assert_eq!(recording.input(key, true), None);
    assert_eq!(recording.input(key, false), Some(key));
}

#[test]
fn ignores_initial_release_and_other_buttons_in_a_chord() {
    let mut recording = Recording::default();
    let first = PushToTalkKey::from_value("163");
    let other = PushToTalkKey::from_value("112");

    assert_eq!(recording.input(other, false), None);
    assert_eq!(recording.input(first, true), None);
    assert_eq!(recording.input(other, true), None);
    assert_eq!(recording.input(other, false), None);
    assert_eq!(recording.input(first, false), Some(first));
}

#[test]
fn preheld_autorepeat_and_release_pass_through_before_fresh_press() {
    let key = PushToTalkKey::from_code(65).unwrap();
    let mut recording = Recording::with_initially_held(vec![key]);

    assert!(!recording.captures(key, true));
    assert_eq!(recording.input(key, true), None);
    assert!(!recording.captures(key, false));
    assert_eq!(recording.input(key, false), None);

    assert!(recording.captures(key, true));
    assert_eq!(recording.input(key, true), None);
    assert_eq!(recording.input(key, false), Some(key));
}
