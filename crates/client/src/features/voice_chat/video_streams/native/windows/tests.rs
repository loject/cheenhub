use super::CanvasMessage;

#[test]
fn canvas_frame_message_preserves_vp9_timing_and_key_frame_metadata() {
    let message = CanvasMessage::Frame {
        timestamp_us: 12_345,
        duration_us: 33_333,
        key_frame: true,
        vp9_base64: "AQID",
    };
    let value = serde_json::to_value(message).expect("canvas frame serializes");

    assert_eq!(value["kind"], "frame");
    assert_eq!(value["timestamp_us"], 12_345);
    assert_eq!(value["duration_us"], 33_333);
    assert_eq!(value["key_frame"], true);
    assert_eq!(value["vp9_base64"], "AQID");
    assert!(value.get("rgba_base64").is_none());
}
