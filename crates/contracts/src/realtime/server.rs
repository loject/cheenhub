//! Server management realtime module contracts.

use serde::{Deserialize, Serialize};

/// Server management realtime message kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerKind {
    /// Load invite links for a server.
    ListServerInvites,
    /// Invite-link list response.
    ServerInviteList,
    /// Revoke one server invite.
    RevokeServerInvite,
    /// Acknowledges that an invite was revoked.
    ServerInviteRevoked,
    /// Kick a member that joined through an invite.
    KickServerInviteMember,
    /// Acknowledges that an invite member was kicked.
    ServerInviteMemberKicked,
}

/// Request payload used to load server invite links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerInvites {
    /// Server identifier.
    pub server_id: String,
}

/// Response payload containing server invite links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteList {
    /// Server identifier.
    pub server_id: String,
    /// Invite links available to the current administrator.
    pub invites: Vec<ServerInviteLink>,
}

/// Server invite link shown in settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteLink {
    /// Stable invite code.
    pub code: String,
    /// User that created the invite.
    pub author_user_id: String,
    /// Current nickname of the invite creator.
    pub author_nickname: String,
    /// Invite creation timestamp in RFC3339 format.
    pub created_at: String,
    /// Optional invite expiration timestamp in RFC3339 format.
    pub expires_at: Option<String>,
    /// Optional maximum number of accepted invite uses.
    pub max_uses: Option<u32>,
    /// Number of successful invite uses.
    pub uses: u32,
    /// Revocation timestamp in RFC3339 format when the invite is revoked.
    pub revoked_at: Option<String>,
    /// Members that joined through this invite.
    pub joined_members: Vec<ServerInviteJoinedMember>,
}

/// Member entry joined through an invite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteJoinedMember {
    /// Stable user identifier.
    pub user_id: String,
    /// Current user nickname.
    pub nickname: String,
    /// Invite-use timestamp in RFC3339 format.
    pub joined_at: String,
    /// Whether the user is currently an active server member.
    pub is_active_member: bool,
}

/// Request payload used to revoke one server invite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeServerInvite {
    /// Server identifier.
    pub server_id: String,
    /// Invite code to revoke.
    pub code: String,
}

/// Response payload returned after revoking one server invite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteRevoked {
    /// Server identifier.
    pub server_id: String,
    /// Revoked invite code.
    pub code: String,
    /// Revocation timestamp in RFC3339 format.
    pub revoked_at: String,
}

/// Request payload used to kick a member that joined through an invite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KickServerInviteMember {
    /// Server identifier.
    pub server_id: String,
    /// Invite code used by the member.
    pub invite_code: String,
    /// User identifier to kick.
    pub user_id: String,
}

/// Response payload returned after kicking an invite member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteMemberKicked {
    /// Server identifier.
    pub server_id: String,
    /// Invite code used by the kicked member.
    pub invite_code: String,
    /// Kicked user identifier.
    pub user_id: String,
}
