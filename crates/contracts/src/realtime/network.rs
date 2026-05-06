//! Network quality realtime module contracts.

use serde::{Deserialize, Serialize};

/// Network module message kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkKind {
    /// Reliable ping request.
    Ping,
    /// Reliable pong response.
    Pong,
}

/// Reliable ping request payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    /// Client-side timestamp in milliseconds.
    pub sent_at_ms: u64,
}

/// Reliable pong response payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    /// Original client-side timestamp in milliseconds.
    pub sent_at_ms: u64,
    /// Server-side receive timestamp in milliseconds.
    pub server_received_at_ms: u64,
    /// Server-side send timestamp in milliseconds.
    pub server_sent_at_ms: u64,
}
