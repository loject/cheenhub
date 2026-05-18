//! Server settings realtime helpers.

use cheenhub_contracts::realtime::{
    KickServerInviteMember, ListServerInvites, RealtimeKind, RealtimeModule, RevokeServerInvite,
    ServerInviteList, ServerInviteMemberKicked, ServerInviteRevoked, ServerKind,
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
