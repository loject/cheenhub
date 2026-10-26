//! Проверки иерархического управления ролями и их назначением.

use cheenhub_contracts::realtime::{ListServerMembers, RevokeServerMemberRole};

use crate::features::servers::application::revoke_server_member_role;

use super::*;

async fn current_drafts(
    state: &AppState,
    server_id: &str,
    manager_id: &Uuid,
) -> Vec<ServerRoleDraft> {
    list_server_roles(
        state,
        manager_id,
        ListServerRoles {
            server_id: server_id.to_owned(),
        },
    )
    .await
    .expect("роли должны быть доступны управляющему")
    .roles
    .into_iter()
    .map(role_draft)
    .collect()
}

async fn create_target_member(state: &AppState, server_id: &str) -> Uuid {
    let server_uuid = Uuid::parse_str(server_id).expect("идентификатор сервера");
    let target_user_id = Uuid::new_v4();
    state
        .server_store
        .insert_server_member(&server_uuid, &target_user_id)
        .await
        .expect("участник должен добавляться");
    target_user_id
}

async fn regular_role_id(state: &AppState, server_id: &str, manager_id: &Uuid) -> String {
    list_server_roles(
        state,
        manager_id,
        ListServerRoles {
            server_id: server_id.to_owned(),
        },
    )
    .await
    .expect("роли должны быть доступны")
    .roles
    .into_iter()
    .find(|role| role.name == "Обычная")
    .expect("обычная роль должна существовать")
    .role_id
}

#[tokio::test]
async fn manager_can_create_lower_regular_role() {
    let (state, server_id, manager_id) = manager_context().await;
    let mut drafts = current_drafts(&state, &server_id, &manager_id).await;
    let member_index = drafts
        .iter()
        .position(|role| role.kind == ServerRoleKind::Member)
        .expect("системная роль должна существовать");
    drafts.insert(
        member_index,
        ServerRoleDraft {
            role_id: None,
            name: "Новая нижняя роль".to_owned(),
            color: "#224466".to_owned(),
            kind: ServerRoleKind::Custom,
            permissions: vec![],
        },
    );

    let saved = save_server_roles(
        &state,
        &manager_id,
        SaveServerRoles {
            server_id,
            roles: drafts,
        },
    )
    .await
    .expect("управляющий должен создавать обычные роли ниже своей");

    assert!(
        saved
            .roles
            .iter()
            .any(|role| role.name == "Новая нижняя роль")
    );
}

#[tokio::test]
async fn manager_can_delete_lower_regular_role() {
    let (state, server_id, manager_id) = manager_context().await;
    let mut drafts = current_drafts(&state, &server_id, &manager_id).await;
    drafts.retain(|role| role.name != "Обычная");

    let saved = save_server_roles(
        &state,
        &manager_id,
        SaveServerRoles {
            server_id,
            roles: drafts,
        },
    )
    .await
    .expect("управляющий должен удалять обычные роли ниже своей");

    assert!(!saved.roles.iter().any(|role| role.name == "Обычная"));
    assert!(saved.roles.iter().any(|role| role.name == "Модератор"));
}

#[tokio::test]
async fn manager_can_list_server_members() {
    let (state, server_id, manager_id) = manager_context().await;
    let listed = list_server_members(&state, &manager_id, ListServerMembers { server_id })
        .await
        .expect("управляющий должен видеть участников для назначения ролей");

    assert!(
        listed
            .members
            .iter()
            .any(|member| member.user_id == manager_id.to_string())
    );
}

#[tokio::test]
async fn manager_can_assign_lower_regular_role() {
    let (state, server_id, manager_id) = manager_context().await;
    let target_user_id = create_target_member(&state, &server_id).await;
    let role_id = regular_role_id(&state, &server_id, &manager_id).await;

    let assigned = assign_server_member_role(
        &state,
        &manager_id,
        AssignServerMemberRole {
            server_id,
            user_id: target_user_id.to_string(),
            role_id: role_id.clone(),
        },
    )
    .await
    .expect("управляющий должен назначать обычную роль ниже себя");

    assert_eq!(assigned.role_id, role_id);
}

#[tokio::test]
async fn manager_can_revoke_lower_regular_role() {
    let (state, server_id, manager_id) = manager_context().await;
    let server_uuid = Uuid::parse_str(&server_id).expect("идентификатор сервера");
    let server = state
        .server_store
        .find_server(&server_uuid)
        .await
        .expect("поиск сервера должен работать")
        .expect("сервер должен существовать");
    let target_user_id = create_target_member(&state, &server_id).await;
    let role_id = regular_role_id(&state, &server_id, &manager_id).await;
    assign_server_member_role(
        &state,
        &server.owner_user_id,
        AssignServerMemberRole {
            server_id: server_id.clone(),
            user_id: target_user_id.to_string(),
            role_id: role_id.clone(),
        },
    )
    .await
    .expect("владелец должен назначить роль для теста");

    let revoked = revoke_server_member_role(
        &state,
        &manager_id,
        RevokeServerMemberRole {
            server_id,
            user_id: target_user_id.to_string(),
            role_id: role_id.clone(),
        },
    )
    .await
    .expect("управляющий должен отзывать обычную роль ниже себя");

    assert_eq!(revoked.role_id, role_id);
}
