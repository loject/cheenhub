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

/// Server list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServersResponse {
    /// Servers available to the current user.
    pub servers: Vec<ServerSummary>,
}
