//! Realtime connection status.

/// Active realtime transport kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RealtimeTransportKind {
    /// Primary WebTransport connection.
    WebTransport,
    /// Slower WebSocket fallback connection.
    WebSocketFallback,
}

/// Current realtime connection state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RealtimeConnectionStatus {
    /// Realtime session is authenticated and ready for requests.
    Connected(RealtimeTransportKind),
    /// Realtime session is not ready.
    Disconnected,
}
