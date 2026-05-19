//! Server management realtime module contracts.

use serde::{Deserialize, Serialize};

/// Server management realtime message kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerKind {
    /// Load members for a server.
    ListServerMembers,
    /// Server-member list response.
    ServerMemberList,
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
    /// Kick an active server member.
    KickServerMember,
    /// Acknowledges that a server member was kicked.
    ServerMemberKicked,
    /// Load roles for a server.
    ListServerRoles,
    /// Server-role list response.
    ServerRoleList,
    /// Save server roles.
    SaveServerRoles,
    /// Acknowledges that server roles were saved.
    ServerRolesSaved,
    /// Assign a custom role to a server member.
    AssignServerMemberRole,
    /// Acknowledges that a role was assigned.
    ServerMemberRoleAssigned,
    /// Revoke a custom role from a server member.
    RevokeServerMemberRole,
    /// Acknowledges that a role was revoked.
    ServerMemberRoleRevoked,
}

/// Request payload used to load server members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerMembers {
    /// Server identifier.
    pub server_id: String,
}

/// Response payload containing active server members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberList {
    /// Server identifier.
    pub server_id: String,
    /// Active members visible to the current administrator.
    pub members: Vec<ServerMemberEntry>,
}

/// Active server member shown in settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberEntry {
    /// Stable user identifier.
    pub user_id: String,
    /// Current user nickname.
    pub nickname: String,
    /// Whether this member owns the server.
    pub is_owner: bool,
    /// Membership start timestamp in RFC3339 format.
    pub joined_at: String,
    /// Invite link used by this member, when available.
    pub invite_code: Option<String>,
    /// Invite-use timestamp in RFC3339 format, when available.
    pub invite_used_at: Option<String>,
    /// Custom role identifiers currently assigned to this member.
    pub role_ids: Vec<String>,
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

/// Request payload used to kick an active server member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KickServerMember {
    /// Server identifier.
    pub server_id: String,
    /// User identifier to kick.
    pub user_id: String,
    /// Optional rejoin block duration in seconds.
    pub exclusion_duration_seconds: Option<u64>,
}

/// Response payload returned after kicking a server member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberKicked {
    /// Server identifier.
    pub server_id: String,
    /// Kicked user identifier.
    pub user_id: String,
    /// Timestamp until which the user cannot rejoin, in RFC3339 format.
    pub excluded_until: Option<String>,
}

/// Request payload used to load server roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerRoles {
    /// Server identifier.
    pub server_id: String,
}

/// Response payload containing server roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleList {
    /// Server identifier.
    pub server_id: String,
    /// Roles ordered from highest to lowest priority.
    pub roles: Vec<ServerRoleEntry>,
}

/// Server role kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerRoleKind {
    /// Mandatory owner role.
    Owner,
    /// Mandatory default member role.
    Member,
    /// User-created role.
    Custom,
}

/// Server role permission flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerRolePermission {
    /// Allows creating server invite links.
    CreateInviteLinks,
    /// Allows kicking members from the server.
    KickServerMembers,
    /// Allows managing server roles.
    ManageRoles,
    /// Allows kicking members from voice rooms.
    KickVoiceMembers,
    /// Allows deleting any message in text rooms.
    DeleteMessages,
}

/// Minimal server role summary embedded in server-level responses for client-side permission checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleSummary {
    /// Stable role identifier.
    pub role_id: String,
    /// Role kind (owner / member / custom).
    pub kind: ServerRoleKind,
    /// Permissions granted by this role.
    pub permissions: Vec<ServerRolePermission>,
}

/// Server role shown in settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleEntry {
    /// Stable role identifier.
    pub role_id: String,
    /// Human-readable role name.
    pub name: String,
    /// Hex role color.
    pub color: String,
    /// Number of members that currently have this role.
    pub members: u32,
    /// Whether this role is mandatory and cannot be deleted.
    pub is_required: bool,
    /// Role kind.
    pub kind: ServerRoleKind,
    /// Effective role permissions.
    pub permissions: Vec<ServerRolePermission>,
}

/// Request payload used to save server roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveServerRoles {
    /// Server identifier.
    pub server_id: String,
    /// Roles ordered from highest to lowest priority.
    pub roles: Vec<ServerRoleDraft>,
}

/// Server role draft sent from settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleDraft {
    /// Existing role identifier. Missing ids create new custom roles.
    pub role_id: Option<String>,
    /// Human-readable role name.
    pub name: String,
    /// Hex role color.
    pub color: String,
    /// Role kind.
    pub kind: ServerRoleKind,
    /// Enabled role permissions.
    pub permissions: Vec<ServerRolePermission>,
}

/// Response payload returned after saving server roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRolesSaved {
    /// Server identifier.
    pub server_id: String,
    /// Saved roles ordered from highest to lowest priority.
    pub roles: Vec<ServerRoleEntry>,
}

/// Request payload used to assign a custom role to a server member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignServerMemberRole {
    /// Server identifier.
    pub server_id: String,
    /// Target user identifier.
    pub user_id: String,
    /// Custom role identifier to assign.
    pub role_id: String,
}

/// Response payload returned after assigning a role to a server member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberRoleAssigned {
    /// Server identifier.
    pub server_id: String,
    /// User that received the role.
    pub user_id: String,
    /// Role that was assigned.
    pub role_id: String,
}

/// Request payload used to revoke a custom role from a server member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeServerMemberRole {
    /// Server identifier.
    pub server_id: String,
    /// Target user identifier.
    pub user_id: String,
    /// Custom role identifier to revoke.
    pub role_id: String,
}

/// Response payload returned after revoking a role from a server member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberRoleRevoked {
    /// Server identifier.
    pub server_id: String,
    /// User that lost the role.
    pub user_id: String,
    /// Role that was revoked.
    pub role_id: String,
}
