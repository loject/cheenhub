//! Server member data for the settings UI.

use cheenhub_contracts::realtime::ServerMemberEntry;

/// Active server member shown in settings.
#[derive(Clone, PartialEq)]
pub(super) struct ServerMemberRow {
    /// Stable user id.
    pub(super) id: String,
    /// Display name.
    pub(super) name: String,
    /// Whether this member owns the server.
    pub(super) is_owner: bool,
    /// Human-readable join time.
    pub(super) joined_at: String,
    /// Invite code used to join the server.
    pub(super) invite_code: Option<String>,
    /// Human-readable invite use time.
    pub(super) invite_used_at: Option<String>,
}

/// Member selected for a kick confirmation.
#[derive(Clone, PartialEq)]
pub(super) struct KickMemberTarget {
    /// Stable user id.
    pub(super) id: String,
    /// Display name.
    pub(super) name: String,
}

/// Converts a realtime member payload into UI data.
pub(super) fn member_from_realtime(member: ServerMemberEntry) -> ServerMemberRow {
    ServerMemberRow {
        id: member.user_id,
        name: member.nickname,
        is_owner: member.is_owner,
        joined_at: member.joined_at,
        invite_code: member.invite_code,
        invite_used_at: member.invite_used_at,
    }
}
