//! Контекст прав текущего пользователя на активном сервере.

use cheenhub_contracts::realtime::{ServerRoleKind, ServerRolePermission};
use cheenhub_contracts::rest::ServerSummary;

/// Описывает действия, доступные текущему пользователю на активном сервере.
#[derive(Clone, Copy, Default)]
pub(crate) struct ServerPermissions {
    /// Может ли пользователь создавать ссылки приглашения.
    pub(crate) can_create_invite_links: bool,
    /// Может ли пользователь исключать участников из голосовых комнат.
    pub(crate) can_kick_voice: bool,
    /// Может ли пользователь удалять чужие сообщения.
    pub(crate) can_delete_messages: bool,
    /// Может ли пользователь создавать, редактировать и удалять комнаты.
    pub(crate) can_manage_rooms: bool,
}

impl ServerPermissions {
    /// Собирает права текущего пользователя из серверной сводки.
    pub(crate) fn from_server(server: &ServerSummary) -> Self {
        Self {
            can_create_invite_links: has_permission(
                server,
                ServerRolePermission::CreateInviteLinks,
            ),
            can_kick_voice: has_permission(server, ServerRolePermission::KickVoiceMembers),
            can_delete_messages: has_permission(server, ServerRolePermission::DeleteMessages),
            can_manage_rooms: has_permission(server, ServerRolePermission::ManageRooms),
        }
    }
}

fn has_permission(server: &ServerSummary, permission: ServerRolePermission) -> bool {
    server.is_owner
        || server.roles.iter().any(|role| {
            ((role.kind == ServerRoleKind::Member && server.is_member)
                || server.member_role_ids.contains(&role.role_id))
                && role.permissions.contains(&permission)
        })
}

#[cfg(test)]
mod tests;
