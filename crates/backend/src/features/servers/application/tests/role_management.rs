//! Проверки иерархического управления серверными ролями.

use cheenhub_contracts::realtime::{
    AssignServerMemberRole, ListServerRoles, SaveServerRoles, ServerRoleDraft, ServerRoleEntry,
    ServerRoleKind, ServerRolePermission,
};

use super::*;

/// Создаёт сервер с управляющей ролью и обычной ролью под ней.
async fn manager_context() -> (AppState, String, Uuid) {
    let state = state();
    let owner_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let server = state
        .server_store
        .insert_server(&owner_id, "Проверка ролей".to_owned())
        .await
        .expect("сервер должен создаваться");
    for member_id in [owner_id, manager_id] {
        state
            .server_store
            .insert_server_member(&server.id, &member_id)
            .await
            .expect("участник должен добавляться");
    }
    let server_id = server.id.to_string();
    let initial = list_server_roles(
        &state,
        &owner_id,
        ListServerRoles {
            server_id: server_id.clone(),
        },
    )
    .await
    .expect("владелец должен видеть роли");
    let mut drafts = initial
        .roles
        .into_iter()
        .map(role_draft)
        .collect::<Vec<_>>();
    drafts.insert(
        1,
        ServerRoleDraft {
            role_id: None,
            name: "Модератор".to_owned(),
            color: "#336699".to_owned(),
            kind: ServerRoleKind::Custom,
            permissions: vec![ServerRolePermission::ManageRoles],
        },
    );
    drafts.insert(
        2,
        ServerRoleDraft {
            role_id: None,
            name: "Обычная".to_owned(),
            color: "#996633".to_owned(),
            kind: ServerRoleKind::Custom,
            permissions: vec![],
        },
    );
    let saved = save_server_roles(
        &state,
        &owner_id,
        SaveServerRoles {
            server_id: server_id.clone(),
            roles: drafts,
        },
    )
    .await
    .expect("владелец должен сохранять роли");
    let manager_role = saved
        .roles
        .iter()
        .find(|role| role.name == "Модератор")
        .expect("управляющая роль должна быть создана");
    assign_server_member_role(
        &state,
        &owner_id,
        AssignServerMemberRole {
            server_id: server_id.clone(),
            user_id: manager_id.to_string(),
            role_id: manager_role.role_id.clone(),
        },
    )
    .await
    .expect("владелец должен назначать управляющую роль");

    (state, server_id, manager_id)
}

fn role_draft(role: ServerRoleEntry) -> ServerRoleDraft {
    ServerRoleDraft {
        role_id: Some(role.role_id),
        name: role.name,
        color: role.color,
        kind: role.kind,
        permissions: role.permissions,
    }
}

#[tokio::test]
async fn member_with_manage_roles_can_list_server_roles() {
    let (state, server_id, manager_id) = manager_context().await;
    let visible = list_server_roles(
        &state,
        &manager_id,
        ListServerRoles {
            server_id: server_id.clone(),
        },
    )
    .await
    .expect("участник с ManageRoles должен видеть роли сервера");

    assert_eq!(visible.server_id, server_id);
    assert!(visible.roles.iter().any(|role| role.name == "Модератор"));
}

#[tokio::test]
async fn manager_can_rename_lower_regular_role() {
    let (state, server_id, manager_id) = manager_context().await;
    let current = list_server_roles(
        &state,
        &manager_id,
        ListServerRoles {
            server_id: server_id.clone(),
        },
    )
    .await
    .expect("управляющий должен видеть список ролей");
    let mut drafts = current
        .roles
        .into_iter()
        .map(role_draft)
        .collect::<Vec<_>>();
    drafts
        .iter_mut()
        .find(|role| role.name == "Обычная")
        .expect("обычная роль должна существовать")
        .name = "Помощник".to_owned();

    let saved = save_server_roles(
        &state,
        &manager_id,
        SaveServerRoles {
            server_id,
            roles: drafts,
        },
    )
    .await
    .expect("управляющий должен редактировать обычную роль ниже себя");

    assert!(saved.roles.iter().any(|role| role.name == "Помощник"));
    assert!(saved.roles.iter().any(|role| role.name == "Модератор"));
}

mod hierarchy;
