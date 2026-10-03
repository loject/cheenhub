use super::parse_input_device_id;

#[test]
fn parses_native_input_device_key() {
    let device_id = "cpal-input:7:Line In: USB";

    assert_eq!(parse_input_device_id(&device_id), Some((7, "Line In: USB")));
}

#[test]
fn rejects_legacy_or_malformed_device_key() {
    assert_eq!(parse_input_device_id("Microphone"), None);
    assert_eq!(parse_input_device_id("cpal-input:name"), None);
    assert_eq!(parse_input_device_id("cpal-input:2:"), None);
}
