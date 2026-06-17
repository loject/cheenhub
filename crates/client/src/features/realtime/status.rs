//! Состояние realtime-соединения.

/// Активный тип realtime-транспорта.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RealtimeTransportKind {
    /// Primary WebTransport connection.
    WebTransport,
    /// Slower WebSocket fallback connection.
    WebSocketFallback,
}

/// Текущее состояние realtime-соединения.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RealtimeConnectionStatus {
    /// Realtime session is authenticated and ready for requests.
    Connected(RealtimeTransportKind),
    /// Realtime session is not ready.
    Disconnected,
}
