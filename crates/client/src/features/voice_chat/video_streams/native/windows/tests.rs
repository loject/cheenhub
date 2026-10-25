use super::{CanvasEvent, CanvasMessage};

#[test]
fn canvas_events_preserve_dimensions_and_decoder_errors() {
    let event: CanvasEvent = serde_json::from_str(r#"{"width":2520,"height":1080}"#).unwrap();
    assert!(matches!(
        event,
        CanvasEvent::Dimensions {
            width: 2520,
            height: 1080
        }
    ));

    let event: CanvasEvent = serde_json::from_str(r#""Decoder failed""#).unwrap();
    assert!(matches!(event, CanvasEvent::Error(message) if message == "Decoder failed"));
}

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
