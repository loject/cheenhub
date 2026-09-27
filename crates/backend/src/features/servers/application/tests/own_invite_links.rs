use cheenhub_contracts::realtime::{
    AssignServerMemberRole, ListServerInvites, ListServerRoles, SaveServerRoles, ServerRoleDraft,
    ServerRoleKind, ServerRolePermission,
};
use cheenhub_contracts::rest::{AuthResponse, ServerSummary};

use super::*;
use crate::features::servers::domain::{MAX_OWN_INVITE_LINKS, ServerInvite, ServerRole};

/// Регистрирует пользователя с уникальным для теста почтовым адресом.
async fn register(state: &AppState, nickname: &str) -> AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: format!("{nickname}@example.com"),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed")
}

/// Создает сервер для указанного пользователя.
async fn create_server(state: &AppState, access_token: &str, name: &str) -> ServerSummary {
    create(
        state,
        access_token,
        CreateServerRequest {
            name: name.to_owned(),
        },
    )
    .await
    .expect("server creation should succeed")
    .server
}

#[tokio::test]
async fn create_invite_returns_own_links_and_limit() {
    let state = state();
    let auth = register(&state, "links_owner").await;
    let server = create_server(&state, &auth.access_token, "Links Hub").await;

    let first = create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    let second = create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(5),
            expires_in_days: None,
        },
    )
    .await
    .expect("second invite creation should succeed");

    assert_eq!(first.limit, MAX_OWN_INVITE_LINKS);
    assert_eq!(first.links.len(), 1);
    assert_eq!(first.links[0].code, first.code);
    assert!(first.links[0].is_active);
    assert_eq!(second.links.len(), 2);
    assert_eq!(second.links[0].code, second.code);
    assert!(second.links.iter().all(|link| link.is_active));
}

#[tokio::test]
async fn list_own_invite_links_returns_only_links_of_current_user() {
    let state = state();
    let owner = register(&state, "links_owner").await;
    let member = register(&state, "links_member").await;
    let server = create_server(&state, &owner.access_token, "Links Hub").await;
    let owner_invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &member.access_token, owner_invite.code.clone())
        .await
        .expect("member should join through the invite");
    grant_create_invite_permission(&state, &owner, &server, &member.user.id).await;
    let member_invite = create_invite(
        &state,
        &member.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("member invite creation should succeed");

    let owner_links = list_own_invite_links(&state, &owner.access_token, server.id.clone())
        .await
        .expect("owner link listing should succeed");
    let member_links = list_own_invite_links(&state, &member.access_token, server.id.clone())
        .await
        .expect("member link listing should succeed");

    assert_eq!(owner_links.links.len(), 1);
    assert_eq!(owner_links.links[0].code, owner_invite.code);
    assert_eq!(member_links.links.len(), 1);
    assert_eq!(member_links.links[0].code, member_invite.code);
    assert_eq!(owner_links.limit, MAX_OWN_INVITE_LINKS);
    assert_eq!(member_links.limit, MAX_OWN_INVITE_LINKS);
}

#[tokio::test]
async fn create_invite_is_rejected_when_link_limit_is_reached() {
    let state = state();
    let auth = register(&state, "limits_owner").await;
    let server = create_server(&state, &auth.access_token, "Limits Hub").await;

    for _ in 0..MAX_OWN_INVITE_LINKS {
        create_invite(
            &state,
            &auth.access_token,
            server.id.clone(),
            CreateServerInviteRequest {
                max_uses: None,
                expires_in_days: None,
            },
        )
        .await
        .expect("invite creation below the limit should succeed");
    }

    let error = create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect_err("invite creation above the limit should be rejected");

    assert!(matches!(error, ServerError::BadRequest(_)));
    assert_eq!(
        server_store_invites(&state, &server).await.len() as u32,
        MAX_OWN_INVITE_LINKS
    );
}

#[tokio::test]
async fn inactive_links_do_not_consume_the_limit() {
    let state = state();
    let auth = register(&state, "exhausted_owner").await;
    let guest = register(&state, "exhausted_guest").await;
    let server = create_server(&state, &auth.access_token, "Exhausted Hub").await;
    let exhausted = create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: Some(1),
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    accept_invite(&state, &guest.access_token, exhausted.code.clone())
        .await
        .expect("guest should join through the invite");

    let response = create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("creation should succeed because the exhausted link frees a slot");

    assert_eq!(response.links.len(), 2);
    assert_eq!(
        response.links.iter().filter(|link| link.is_active).count() as u32,
        1
    );
}

#[tokio::test]
async fn user_can_delete_own_invite_link() {
    let state = state();
    let auth = register(&state, "delete_owner").await;
    let server = create_server(&state, &auth.access_token, "Delete Hub").await;
    let first = create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    create_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("second invite creation should succeed");

    let response = delete_own_invite(
        &state,
        &auth.access_token,
        server.id.clone(),
        first.code.clone(),
    )
    .await
    .expect("own invite deletion should succeed");

    assert_eq!(response.code, first.code);
    assert_eq!(response.links.len(), 1);
    assert_ne!(response.links[0].code, first.code);
    assert_eq!(response.limit, MAX_OWN_INVITE_LINKS);

    // Строка приглашения сохраняется с отметкой удаления, чтобы не терять источник входа.
    let stored = server_store_invites(&state, &server).await;
    assert_eq!(stored.len(), 2);
    let deleted = stored
        .iter()
        .find(|invite| invite.id.to_string() == first.code)
        .expect("deleted invite should stay stored");
    let kept = stored
        .iter()
        .find(|invite| invite.id.to_string() != first.code)
        .expect("other invite should stay stored");
    assert!(deleted.deleted_at.is_some());
    assert!(kept.deleted_at.is_none());
}

#[tokio::test]
async fn deleted_invite_link_stops_working_but_keeps_join_attribution() {
    let state = state();
    let owner = register(&state, "attribution_owner").await;
    let member = register(&state, "attribution_member").await;
    let latecomer = register(&state, "attribution_latecomer").await;
    let server = create_server(&state, &owner.access_token, "Attribution Hub").await;
    let invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    accept_invite(&state, &member.access_token, invite.code.clone())
        .await
        .expect("member should join through the invite");
    delete_own_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        invite.code.clone(),
    )
    .await
    .expect("own invite deletion should succeed");

    // Удалённая ссылка больше не пускает новых участников.
    let error = accept_invite(&state, &latecomer.access_token, invite.code.clone())
        .await
        .expect_err("deleted invite should not accept new members");
    assert!(matches!(error, ServerError::NotFound(_)));

    // Владелец по-прежнему видит ссылку и знает, кто по ней пришёл.
    let owner_id = Uuid::parse_str(&owner.user.id).expect("owner id should be uuid");
    let listed = list_server_invites(
        &state,
        &owner_id,
        ListServerInvites {
            server_id: server.id.clone(),
        },
    )
    .await
    .expect("invite listing should succeed");
    let stored = listed
        .invites
        .iter()
        .find(|stored| stored.code == invite.code)
        .expect("deleted invite should stay visible in server settings");

    assert!(stored.deleted_at.is_some());
    assert_eq!(stored.joined_members.len(), 1);
    assert_eq!(stored.joined_members[0].user_id, member.user.id);
}

#[tokio::test]
async fn user_cannot_delete_invite_link_created_by_someone_else() {
    let state = state();
    let owner = register(&state, "foreign_owner").await;
    let member = register(&state, "foreign_member").await;
    let server = create_server(&state, &owner.access_token, "Foreign Hub").await;
    let owner_invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &member.access_token, owner_invite.code.clone())
        .await
        .expect("member should join through the invite");
    grant_create_invite_permission(&state, &owner, &server, &member.user.id).await;

    let error = delete_own_invite(
        &state,
        &member.access_token,
        server.id.clone(),
        owner_invite.code.clone(),
    )
    .await
    .expect_err("deleting a foreign invite link should be rejected");

    assert!(matches!(error, ServerError::NotFound(_)));
    let owner_links = list_own_invite_links(&state, &owner.access_token, server.id.clone())
        .await
        .expect("owner link listing should succeed");
    assert_eq!(owner_links.links.len(), 1);
}

#[tokio::test]
async fn own_invite_links_are_not_available_without_invite_permission() {
    let state = state();
    let owner = register(&state, "denied_owner").await;
    let guest = register(&state, "denied_guest").await;
    let server = create_server(&state, &owner.access_token, "Denied Hub").await;
    let owner_invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("owner invite creation should succeed");
    accept_invite(&state, &guest.access_token, owner_invite.code.clone())
        .await
        .expect("guest should join through the invite");
    remove_create_invite_permission(&state, &server).await;

    let error = list_own_invite_links(&state, &guest.access_token, server.id.clone())
        .await
        .expect_err("listing without invite permission should be rejected");

    assert!(matches!(error, ServerError::NotFound(_)));
}

/// Возвращает все приглашения сервера напрямую из хранилища.
async fn server_store_invites(state: &AppState, server: &ServerSummary) -> Vec<ServerInvite> {
    let server_id = Uuid::parse_str(&server.id).expect("server id should be uuid");

    state
        .server_store
        .list_server_invites(&server_id)
        .await
        .expect("invites should be readable")
}

/// Убирает право создавать ссылки приглашения у всех ролей сервера.
async fn remove_create_invite_permission(state: &AppState, server: &ServerSummary) {
    let server_id = Uuid::parse_str(&server.id).expect("server id should be uuid");
    let roles = state
        .server_store
        .list_server_roles(&server_id)
        .await
        .expect("roles should be readable");
    let roles = roles
        .into_iter()
        .map(|role| ServerRole {
            permissions: role
                .permissions
                .into_iter()
                .filter(|permission| *permission != ServerRolePermission::CreateInviteLinks)
                .collect(),
            ..role
        })
        .collect();
    state
        .server_store
        .replace_server_roles(&server_id, roles)
        .await
        .expect("roles should be replaceable");
}

/// Выдает участнику роль с правом создавать ссылки приглашения.
async fn grant_create_invite_permission(
    state: &AppState,
    owner: &AuthResponse,
    server: &ServerSummary,
    user_id: &str,
) {
    let owner_id = Uuid::parse_str(&owner.user.id).expect("owner id should be uuid");
    let role_list = list_server_roles(
        state,
        &owner_id,
        ListServerRoles {
            server_id: server.id.clone(),
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
    drafts.push(ServerRoleDraft {
        role_id: None,
        name: "Инвайты".to_owned(),
        color: "#38bdf8".to_owned(),
        kind: ServerRoleKind::Custom,
        permissions: vec![ServerRolePermission::CreateInviteLinks],
    });
    let saved_roles = save_server_roles(
        state,
        &owner_id,
        SaveServerRoles {
            server_id: server.id.clone(),
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
        state,
        &owner_id,
        AssignServerMemberRole {
            server_id: server.id.clone(),
            user_id: user_id.to_owned(),
            role_id: invite_role_id,
        },
    )
    .await
    .expect("role should be assigned");
}
