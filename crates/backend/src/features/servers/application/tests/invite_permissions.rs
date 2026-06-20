use cheenhub_contracts::realtime::{
    AssignServerMemberRole, ListServerRoles, SaveServerRoles, ServerRoleDraft, ServerRoleKind,
    ServerRolePermission,
};

use super::*;

#[tokio::test]
async fn member_with_invite_permission_can_create_server_invite() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_role_owner".to_owned(),
            email: "invite-role-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let member_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_role_member".to_owned(),
            email: "invite-role-member@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("member registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Invite Role Hub".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let server_id = server.server.id.clone();
    let owner_invite = create_invite(
        &state,
        &owner_auth.access_token,
        server_id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &member_auth.access_token, owner_invite.code)
        .await
        .expect("member should join");
    let owner_id = Uuid::parse_str(&owner_auth.user.id).expect("owner id should be uuid");
    let role_list = list_server_roles(
        &state,
        &owner_id,
        ListServerRoles {
            server_id: server_id.clone(),
        },
    )
    .await
    .expect("roles should load");
    let mut drafts = role_list
        .roles
        .into_iter()
        .map(|role| ServerRoleDraft {
            role_id: Some(role.role_id),
            name: role.name,
            color: role.color,
            kind: role.kind,
            permissions: role.permissions,
        })
        .collect::<Vec<_>>();
    drafts.insert(
        1,
        ServerRoleDraft {
            role_id: None,
            name: "Инвайты".to_owned(),
            color: "#38bdf8".to_owned(),
            kind: ServerRoleKind::Custom,
            permissions: vec![ServerRolePermission::CreateInviteLinks],
        },
    );
    let saved_roles = save_server_roles(
        &state,
        &owner_id,
        SaveServerRoles {
            server_id: server_id.clone(),
            roles: drafts,
        },
    )
    .await
    .expect("roles should save");
    let invite_role_id = saved_roles
        .roles
        .iter()
        .find(|role| role.kind == ServerRoleKind::Custom && role.name == "Инвайты")
        .expect("custom invite role should be saved")
        .role_id
        .clone();
    assign_server_member_role(
        &state,
        &owner_id,
        AssignServerMemberRole {
            server_id: server_id.clone(),
            user_id: member_auth.user.id.clone(),
            role_id: invite_role_id,
        },
    )
    .await
    .expect("role should be assigned");

    let response = create_invite(
        &state,
        &member_auth.access_token,
        server_id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(2),
            expires_in_days: None,
        },
    )
    .await
    .expect("member with invite permission should create invite");
    let invites = server_store
        .invites_for_tests()
        .expect("invites should be readable");

    assert_eq!(invites.len(), 2);
    let created_invite = invites
        .iter()
        .find(|invite| invite.id.to_string() == response.code)
        .expect("created invite should be stored");
    assert_eq!(created_invite.server_id.to_string(), server_id);
    assert_eq!(
        created_invite.creator_user_id.to_string(),
        member_auth.user.id
    );
    assert_eq!(created_invite.max_uses, Some(2));
}

#[tokio::test]
async fn member_role_invite_permission_allows_regular_member_to_create_server_invite() {
    let server_store = Arc::new(InMemoryServerStore::default());
    let state = state_with_store(server_store.clone());
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "member_role_invite_owner".to_owned(),
            email: "member-role-invite-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let member_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "member_role_invite_member".to_owned(),
            email: "member-role-invite-member@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("member registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Member Role Invite Hub".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let server_id = server.server.id.clone();
    let owner_invite = create_invite(
        &state,
        &owner_auth.access_token,
        server_id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &member_auth.access_token, owner_invite.code)
        .await
        .expect("member should join");
    let owner_id = Uuid::parse_str(&owner_auth.user.id).expect("owner id should be uuid");
    let role_list = list_server_roles(
        &state,
        &owner_id,
        ListServerRoles {
            server_id: server_id.clone(),
        },
    )
    .await
    .expect("roles should load");
    let drafts = role_list
        .roles
        .into_iter()
        .map(|role| {
            let permissions = if role.kind == ServerRoleKind::Member {
                vec![ServerRolePermission::CreateInviteLinks]
            } else {
                role.permissions
            };
            ServerRoleDraft {
                role_id: Some(role.role_id),
                name: role.name,
                color: role.color,
                kind: role.kind,
                permissions,
            }
        })
        .collect::<Vec<_>>();
    save_server_roles(
        &state,
        &owner_id,
        SaveServerRoles {
            server_id: server_id.clone(),
            roles: drafts,
        },
    )
    .await
    .expect("roles should save");

    let response = create_invite(
        &state,
        &member_auth.access_token,
        server_id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(4),
            expires_in_days: None,
        },
    )
    .await
    .expect("member role permission should allow invite creation");
    let invites = server_store
        .invites_for_tests()
        .expect("invites should be readable");

    assert_eq!(invites.len(), 2);
    let created_invite = invites
        .iter()
        .find(|invite| invite.id.to_string() == response.code)
        .expect("created invite should be stored");
    assert_eq!(created_invite.server_id.to_string(), server_id);
    assert_eq!(
        created_invite.creator_user_id.to_string(),
        member_auth.user.id
    );
    assert_eq!(created_invite.max_uses, Some(4));
}

#[tokio::test]
async fn member_without_invite_permission_cannot_create_server_invite() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_denied_owner".to_owned(),
            email: "invite-denied-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let member_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invite_denied_member".to_owned(),
            email: "invite-denied-member@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("member registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Invite Denied Hub".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let owner_invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &member_auth.access_token, owner_invite.code)
        .await
        .expect("member should join");

    let result = create_invite(
        &state,
        &member_auth.access_token,
        server.server.id,
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await;

    assert!(matches!(result, Err(ServerError::NotFound(_))));
}
