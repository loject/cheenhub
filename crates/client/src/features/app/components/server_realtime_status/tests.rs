use super::*;

#[test]
fn describes_each_realtime_connection_attempt() {
    assert_eq!(
        realtime_connection_status_label(RealtimeConnectionStatus::ConnectingWebTransport),
        "Подключение…"
    );
    assert_eq!(
        realtime_connection_status_label(RealtimeConnectionStatus::ConnectingWebSocketFallback),
        "Подключение через WebSocket fallback…"
    );
}
