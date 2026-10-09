use super::*;

#[test]
fn invalid_binding_uses_safe_default() {
    for value in ["", "garbage", "65535", "0", "255", "-1"] {
        assert_eq!(PushToTalkKey::from_value(value), PushToTalkKey::default());
    }
}

#[test]
fn supported_binding_survives_storage_roundtrip() {
    for code in 1..=254 {
        assert_eq!(PushToTalkKey::from_value(&code.to_string()).code(), code);
    }
}

#[test]
fn records_letters_mouse_buttons_and_extended_function_keys() {
    for code in [1, 2, 4, 5, 6, 0x41, 0x5a, 0x87, 0xb3, 0xe2] {
        assert_eq!(PushToTalkKey::from_value(&code.to_string()).code(), code);
    }
}

#[test]
fn side_mouse_buttons_have_familiar_names() {
    assert_eq!(PushToTalkKey::from_value("5").fallback_label(), "Mouse4");
    assert_eq!(PushToTalkKey::from_value("6").fallback_label(), "Mouse5");
}
