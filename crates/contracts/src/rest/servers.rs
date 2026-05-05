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

/// Server invite data returned by invite lookup endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteSummary {
    /// Stable invite code.
    pub code: String,
    /// Optional maximum number of accepted invite uses.
    pub max_uses: Option<u32>,
    /// Optional invite expiration timestamp in RFC3339 format.
    pub expires_at: Option<String>,
}

/// Successful server invite lookup response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteInfoResponse {
    /// Invite metadata.
    pub invite: ServerInviteSummary,
    /// Server the invite points to.
    pub server: ServerSummary,
}

/// Server list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServersResponse {
    /// Servers available to the current user.
    pub servers: Vec<ServerSummary>,
}
