//! Server REST contracts.

use serde::{Deserialize, Serialize};

/// Request body used to create a new server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerRequest {
    /// Human-readable server name.
    pub name: String,
}

/// Server data returned by server endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSummary {
    /// Stable server identifier.
    pub id: String,
    /// Human-readable server name.
    pub name: String,
    /// Whether the current user owns the server.
    pub is_owner: bool,
}

/// Successful server creation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerResponse {
    /// Created server.
    pub server: ServerSummary,
}

/// Request body used to create a server invite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerInviteRequest {
    /// Optional maximum number of accepted invite uses.
    pub max_uses: Option<u32>,
    /// Optional invite lifetime in days.
    pub expires_in_days: Option<u32>,
}

/// Successful server invite creation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerInviteResponse {
    /// Stable invite code.
    pub code: String,
}

/// Server list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServersResponse {
    /// Servers available to the current user.
    pub servers: Vec<ServerSummary>,
}
