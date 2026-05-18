//! Postgres row conversion helpers for server infrastructure.

use cheenhub_contracts::realtime::ServerRoleKind;
use cheenhub_contracts::rest::ServerRoomKind;

use crate::features::servers::domain::{
    Server, ServerInvite, ServerInviteUse, ServerMember, ServerMemberExclusion, ServerRole,
    ServerRoom,
};
use crate::features::servers::infrastructure::entities::{
    server_invite_uses, server_invites, server_member_exclusions, server_members, server_roles,
    server_rooms, servers,
};

pub(super) fn server_room_from_model(row: server_rooms::Model) -> anyhow::Result<ServerRoom> {
    let position = row.position.try_into().unwrap_or(0);

    Ok(ServerRoom {
        id: row.id,
        server_id: row.server_id,
        name: row.name,
        kind: room_kind_from_str(&row.kind)?,
        position,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub(super) fn room_kind_as_str(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "text",
        ServerRoomKind::Voice => "voice",
        ServerRoomKind::TextAndVoice => "text_and_voice",
    }
}

pub(super) fn server_role_from_model(
    row: server_roles::Model,
    permissions: Vec<cheenhub_contracts::realtime::ServerRolePermission>,
) -> anyhow::Result<ServerRole> {
    let position = row.position.try_into().unwrap_or(0);

    Ok(ServerRole {
        id: row.id,
        server_id: row.server_id,
        name: row.name,
        color: row.color,
        kind: role_kind_from_str(&row.kind)?,
        position,
        permissions,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub(super) fn role_kind_as_str(kind: ServerRoleKind) -> &'static str {
    match kind {
        ServerRoleKind::Owner => "owner",
        ServerRoleKind::Member => "member",
        ServerRoleKind::Custom => "custom",
    }
}

pub(super) fn role_permission_as_str(
    permission: cheenhub_contracts::realtime::ServerRolePermission,
) -> &'static str {
    match permission {
        cheenhub_contracts::realtime::ServerRolePermission::CreateInviteLinks => {
            "create_invite_links"
        }
        cheenhub_contracts::realtime::ServerRolePermission::KickServerMembers => {
            "kick_server_members"
        }
        cheenhub_contracts::realtime::ServerRolePermission::ManageRoles => "manage_roles",
        cheenhub_contracts::realtime::ServerRolePermission::KickVoiceMembers => {
            "kick_voice_members"
        }
    }
}

fn room_kind_from_str(kind: &str) -> anyhow::Result<ServerRoomKind> {
    match kind {
        "text" => Ok(ServerRoomKind::Text),
        "voice" => Ok(ServerRoomKind::Voice),
        "text_and_voice" => Ok(ServerRoomKind::TextAndVoice),
        other => Err(anyhow::anyhow!("unknown server room kind: {other}")),
    }
}

fn role_kind_from_str(kind: &str) -> anyhow::Result<ServerRoleKind> {
    match kind {
        "owner" => Ok(ServerRoleKind::Owner),
        "member" => Ok(ServerRoleKind::Member),
        "custom" => Ok(ServerRoleKind::Custom),
        other => Err(anyhow::anyhow!("unknown server role kind: {other}")),
    }
}

pub(super) fn role_permission_from_str(
    permission: &str,
) -> anyhow::Result<cheenhub_contracts::realtime::ServerRolePermission> {
    match permission {
        "create_invite_links" => {
            Ok(cheenhub_contracts::realtime::ServerRolePermission::CreateInviteLinks)
        }
        "kick_server_members" => {
            Ok(cheenhub_contracts::realtime::ServerRolePermission::KickServerMembers)
        }
        "manage_roles" => Ok(cheenhub_contracts::realtime::ServerRolePermission::ManageRoles),
        "kick_voice_members" => {
            Ok(cheenhub_contracts::realtime::ServerRolePermission::KickVoiceMembers)
        }
        other => Err(anyhow::anyhow!("unknown server role permission: {other}")),
    }
}

impl From<servers::Model> for Server {
    fn from(row: servers::Model) -> Self {
        Self {
            id: row.id,
            owner_user_id: row.owner_user_id,
            name: row.name,
            avatar_image_id: row.avatar_image_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<server_invites::Model> for ServerInvite {
    fn from(row: server_invites::Model) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            creator_user_id: row.creator_user_id,
            max_uses: row.max_uses.map(|value| value as u32),
            expires_at: row.expires_at,
            created_at: row.created_at,
            revoked_at: row.revoked_at,
        }
    }
}

impl From<server_members::Model> for ServerMember {
    fn from(row: server_members::Model) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            user_id: row.user_id,
            joined_at: row.joined_at,
            left_at: row.left_at,
        }
    }
}

impl From<server_member_exclusions::Model> for ServerMemberExclusion {
    fn from(row: server_member_exclusions::Model) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            user_id: row.user_id,
            initiator_user_id: row.initiator_user_id,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

impl From<server_invite_uses::Model> for ServerInviteUse {
    fn from(row: server_invite_uses::Model) -> Self {
        Self {
            id: row.id,
            invite_id: row.invite_id,
            user_id: row.user_id,
            used_at: row.used_at,
        }
    }
}
