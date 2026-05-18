//! Server settings realtime helpers.

use cheenhub_contracts::realtime::{
    KickServerInviteMember, KickServerMember, ListServerInvites, ListServerMembers,
    ListServerRoles, RealtimeKind, RealtimeModule, RevokeServerInvite, SaveServerRoles,
    ServerInviteList, ServerInviteMemberKicked, ServerInviteRevoked, ServerKind,
    ServerMemberKicked, ServerMemberList, ServerRoleDraft, ServerRoleList, ServerRolesSaved,
};

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Loads server invite links through the realtime session.
pub(super) async fn list_server_invites(
    realtime: &RealtimeHandle,
    server_id: String,
) -> Result<ServerInviteList, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::ListServerInvites),
            ListServerInvites { server_id },
        )
        .await
}

/// Loads server members through the realtime session.
pub(super) async fn list_server_members(
    realtime: &RealtimeHandle,
    server_id: String,
) -> Result<ServerMemberList, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::ListServerMembers),
            ListServerMembers { server_id },
        )
        .await
}

/// Revokes one server invite through the realtime session.
pub(super) async fn revoke_server_invite(
    realtime: &RealtimeHandle,
    server_id: String,
    code: String,
) -> Result<ServerInviteRevoked, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::RevokeServerInvite),
            RevokeServerInvite { server_id, code },
        )
        .await
}

/// Kicks a member that joined through a server invite.
pub(super) async fn kick_server_invite_member(
    realtime: &RealtimeHandle,
    server_id: String,
    invite_code: String,
    user_id: String,
) -> Result<ServerInviteMemberKicked, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::KickServerInviteMember),
            KickServerInviteMember {
                server_id,
                invite_code,
                user_id,
            },
        )
        .await
}

/// Kicks an active server member.
pub(super) async fn kick_server_member(
    realtime: &RealtimeHandle,
    server_id: String,
    user_id: String,
    exclusion_duration_seconds: Option<u64>,
) -> Result<ServerMemberKicked, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::KickServerMember),
            KickServerMember {
                server_id,
                user_id,
                exclusion_duration_seconds,
            },
        )
        .await
}

/// Loads server roles through the realtime session.
pub(super) async fn list_server_roles(
    realtime: &RealtimeHandle,
    server_id: String,
) -> Result<ServerRoleList, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::ListServerRoles),
            ListServerRoles { server_id },
        )
        .await
}

/// Saves server roles through the realtime session.
pub(super) async fn save_server_roles(
    realtime: &RealtimeHandle,
    server_id: String,
    roles: Vec<ServerRoleDraft>,
) -> Result<ServerRolesSaved, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::SaveServerRoles),
            SaveServerRoles { server_id, roles },
        )
        .await
}
