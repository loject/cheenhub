use super::*;

#[test]
fn classifies_common_transport_failures() {
    assert_eq!(
        classify_transport_failure(&RealtimeError::new("DNS lookup failed")),
        WebTransportFallbackReason::Dns
    );
    assert_eq!(
        classify_transport_failure(&RealtimeError::new("UnknownIssuer certificate error")),
        WebTransportFallbackReason::Tls
    );
    assert_eq!(
        classify_transport_failure(&RealtimeError::new("WebTransport connection rejected")),
        WebTransportFallbackReason::Transport
    );
    assert_eq!(
        classify_transport_failure(&RealtimeError::new("QUIC connection closed")),
        WebTransportFallbackReason::Transport
    );
}
