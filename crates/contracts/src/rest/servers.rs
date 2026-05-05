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
    /// Whether the current user is an active server member.
    pub is_member: bool,
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
    /// Number of successful invite uses.
    pub uses: u32,
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

/// Successful server invite acceptance response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptServerInviteResponse {
    /// Server the current user can now access.
    pub server: ServerSummary,
    /// Whether the current user was already an active member.
    pub already_member: bool,
}

/// Server list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServersResponse {
    /// Servers available to the current user.
    pub servers: Vec<ServerSummary>,
}
