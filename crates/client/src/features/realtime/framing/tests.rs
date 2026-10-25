use super::{InboundRealtimeEnvelope, decode_inbound_envelope};

#[test]
fn unknown_kind_is_skipped() {
    let input =
        br#"{"module":"text_chat","kind":"future_message_kind","request_id":null,"payload":{}}"#;
    let decoded = decode_inbound_envelope(input).expect("valid unknown event");
    assert!(matches!(
        decoded,
        InboundRealtimeEnvelope::Unknown { module, kind }
            if module == "text_chat" && kind == "future_message_kind"
    ));
}

#[test]
fn unknown_module_is_skipped() {
    let input =
        br#"{"module":"future_module","kind":"control_ack","request_id":null,"payload":{}}"#;
    let decoded = decode_inbound_envelope(input).expect("valid unknown module");
    assert!(matches!(
        decoded,
        InboundRealtimeEnvelope::Unknown { module, kind }
            if module == "future_module" && kind == "control_ack"
    ));
}

#[test]
fn known_event_is_decoded() {
    let input = br#"{"module":"control","kind":"control_ack","request_id":null,"payload":{}}"#;
    let decoded = decode_inbound_envelope(input).expect("valid event");
    assert!(
        matches!(decoded, InboundRealtimeEnvelope::Known(envelope) if envelope.has_matching_module_kind())
    );
}

#[test]
fn malformed_envelope_is_not_silently_skipped() {
    assert!(decode_inbound_envelope(br#"{"module":"control""#).is_err());
    assert!(decode_inbound_envelope(br#"{"module":"control","kind":42,"payload":{}}"#).is_err());
    assert!(
        decode_inbound_envelope(
            br#"{"module":"control","kind":"future_kind","request_id":"invalid","payload":{}}"#
        )
        .is_err()
    );
}
