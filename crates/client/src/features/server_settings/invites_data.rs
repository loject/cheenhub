//! Server invite data for the settings UI.

use cheenhub_contracts::realtime::ServerInviteLink;

/// Server invite availability status.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum InviteStatus {
    /// Invite can be used to join the server.
    Active,
    /// Invite was revoked and can no longer be used.
    Revoked,
}

/// Server invite link shown in settings.
#[derive(Clone, PartialEq)]
pub(super) struct InviteLink {
    /// Stable local invite row id.
    pub(super) id: String,
    /// Invite code shown to administrators.
    pub(super) code: String,
    /// Display name of the invite creator.
    pub(super) author: String,
    /// Human-readable creation time.
    pub(super) created_at: String,
    /// Human-readable expiration time.
    pub(super) expires_at: String,
    /// Optional usage limit.
    pub(super) max_uses: Option<u32>,
    /// Current invite status.
    pub(super) status: InviteStatus,
    /// Members who joined through this invite.
    pub(super) joined_members: Vec<InviteJoin>,
}

/// Member entry joined through an invite.
#[derive(Clone, PartialEq)]
pub(super) struct InviteJoin {
    /// Stable member id.
    pub(super) id: String,
    /// Member display name.
    pub(super) name: String,
    /// Human-readable join time.
    pub(super) joined_at: String,
    /// Whether this member can currently be kicked.
    pub(super) is_active_member: bool,
}

/// Converts a realtime invite payload into UI data.
pub(super) fn invite_from_realtime(invite: ServerInviteLink) -> InviteLink {
    InviteLink {
        id: invite.code.clone(),
        code: invite.code,
        author: invite.author_nickname,
        created_at: invite.created_at,
        expires_at: invite.expires_at.unwrap_or_else(|| "без срока".to_owned()),
        max_uses: invite.max_uses,
        status: if invite.revoked_at.is_some() {
            InviteStatus::Revoked
        } else {
            InviteStatus::Active
        },
        joined_members: invite
            .joined_members
            .into_iter()
            .map(|member| InviteJoin {
                id: member.user_id,
                name: member.nickname,
                joined_at: member.joined_at,
                is_active_member: member.is_active_member,
            })
            .collect(),
    }
}
