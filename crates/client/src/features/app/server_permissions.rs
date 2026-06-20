//! Контекст прав текущего пользователя на активном сервере.

use cheenhub_contracts::realtime::{ServerRoleKind, ServerRolePermission};
use cheenhub_contracts::rest::ServerSummary;

/// Описывает действия, доступные текущему пользователю на активном сервере.
#[derive(Clone, Copy)]
pub(crate) struct ServerPermissionsContext {
    /// Может ли пользователь создавать ссылки приглашения.
    pub(crate) can_create_invite_links: bool,
    /// Может ли пользователь исключать участников из голосовых комнат.
    pub(crate) can_kick_voice: bool,
    /// Может ли пользователь удалять чужие сообщения.
    pub(crate) can_delete_messages: bool,
}

impl ServerPermissionsContext {
    /// Собирает права текущего пользователя из серверной сводки.
    pub(crate) fn from_server(server: &ServerSummary) -> Self {
        Self {
            can_create_invite_links: has_permission(
                server,
                ServerRolePermission::CreateInviteLinks,
            ),
            can_kick_voice: has_permission(server, ServerRolePermission::KickVoiceMembers),
            can_delete_messages: has_permission(server, ServerRolePermission::DeleteMessages),
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
mod tests {
    use cheenhub_contracts::realtime::ServerRoleSummary;

    use super::*;

    #[test]
    fn member_role_permission_applies_to_active_member_without_assigned_role_id() {
        let permissions = ServerPermissionsContext::from_server(&server_summary(
            true,
            vec![role(
                "member-role",
                ServerRoleKind::Member,
                vec![ServerRolePermission::CreateInviteLinks],
            )],
            Vec::new(),
        ));

        assert!(permissions.can_create_invite_links);
    }

    #[test]
    fn member_role_permission_does_not_apply_to_non_member_preview() {
        let permissions = ServerPermissionsContext::from_server(&server_summary(
            false,
            vec![role(
                "member-role",
                ServerRoleKind::Member,
                vec![ServerRolePermission::CreateInviteLinks],
            )],
            Vec::new(),
        ));

        assert!(!permissions.can_create_invite_links);
    }

    #[test]
    fn custom_role_permission_requires_assigned_role_id() {
        let role = role(
            "custom-role",
            ServerRoleKind::Custom,
            vec![ServerRolePermission::CreateInviteLinks],
        );

        let without_role = ServerPermissionsContext::from_server(&server_summary(
            true,
            vec![role.clone()],
            Vec::new(),
        ));
        let with_role = ServerPermissionsContext::from_server(&server_summary(
            true,
            vec![role],
            vec!["custom-role".to_owned()],
        ));

        assert!(!without_role.can_create_invite_links);
        assert!(with_role.can_create_invite_links);
    }

    fn server_summary(
        is_member: bool,
        roles: Vec<ServerRoleSummary>,
        member_role_ids: Vec<String>,
    ) -> ServerSummary {
        ServerSummary {
            id: "server-id".to_owned(),
            name: "Server".to_owned(),
            avatar_url: None,
            is_owner: false,
            is_member,
            roles,
            member_role_ids,
        }
    }

    fn role(
        role_id: &str,
        kind: ServerRoleKind,
        permissions: Vec<ServerRolePermission>,
    ) -> ServerRoleSummary {
        ServerRoleSummary {
            role_id: role_id.to_owned(),
            kind,
            permissions,
        }
    }
}
