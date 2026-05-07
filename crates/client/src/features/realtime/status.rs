//! Realtime connection status.

/// Current WebTransport realtime connection state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RealtimeConnectionStatus {
    /// Realtime session is authenticated and ready for requests.
    Connected,
    /// Realtime session is not ready.
    Disconnected,
}
