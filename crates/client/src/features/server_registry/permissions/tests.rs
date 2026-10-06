use cheenhub_contracts::realtime::ServerRoleSummary;

use super::*;

#[test]
fn member_role_permission_applies_to_active_member_without_assigned_role_id() {
    let permissions = ServerPermissions::from_server(&server_summary(
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
    let permissions = ServerPermissions::from_server(&server_summary(
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
    let without_role =
        ServerPermissions::from_server(&server_summary(true, vec![role.clone()], Vec::new()));
    let with_role = ServerPermissions::from_server(&server_summary(
        true,
        vec![role],
        vec!["custom-role".to_owned()],
    ));

    assert!(!without_role.can_create_invite_links);
    assert!(with_role.can_create_invite_links);
}

#[test]
fn manage_rooms_permission_grants_room_management() {
    let permissions = ServerPermissions::from_server(&server_summary(
        true,
        vec![role(
            "rooms-manager",
            ServerRoleKind::Custom,
            vec![ServerRolePermission::ManageRooms],
        )],
        vec!["rooms-manager".to_owned()],
    ));

    assert!(permissions.can_manage_rooms);
}

#[test]
fn room_management_is_denied_without_permission() {
    let permissions = ServerPermissions::from_server(&server_summary(
        true,
        vec![role(
            "plain-member",
            ServerRoleKind::Custom,
            vec![ServerRolePermission::DeleteMessages],
        )],
        vec!["plain-member".to_owned()],
    ));

    assert!(!permissions.can_manage_rooms);
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
        name: role_id.to_owned(),
        color: "#3b82f6".to_owned(),
        kind,
        permissions,
    }
}
