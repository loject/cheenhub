//! Проверка серверного снимка ролей без добавления данных в сообщения.

use cheenhub_contracts::realtime::ServerRoleKind;
use uuid::Uuid;

use super::state;
use crate::features::servers::domain::ServerRole;

fn role(server_id: Uuid, kind: ServerRoleKind, position: u32, color: &str) -> ServerRole {
    let now = chrono::Utc::now();
    ServerRole {
        id: Uuid::new_v4(),
        server_id,
        name: format!("Роль {position}"),
        color: color.to_owned(),
        kind,
        position,
        permissions: Vec::new(),
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn server_summary_includes_current_members_roles_and_hides_them_from_guests() {
    let state = state();
    let owner_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    let outsider_id = Uuid::new_v4();
    let server = state
        .server_store
        .insert_server(&owner_id, "Ролевой сервер".to_owned())
        .await
        .unwrap();
    state
        .server_store
        .insert_server_member(&server.id, &member_id)
        .await
        .unwrap();
    let owner_role = role(server.id, ServerRoleKind::Owner, 0, "#facc15");
    let custom_role = role(server.id, ServerRoleKind::Custom, 1, "#ef4444");
    let member_role = role(server.id, ServerRoleKind::Member, 2, "#94a3b8");
    state
        .server_store
        .replace_server_roles(
            &server.id,
            vec![member_role.clone(), custom_role.clone(), owner_role.clone()],
        )
        .await
        .unwrap();
    state
        .server_store
        .assign_server_member_role(&server.id, &member_id, &custom_role.id, &owner_id)
        .await
        .unwrap();

    let summary = super::super::support::server_summary(&state, &server, &member_id, true).await;
    assert_eq!(
        summary
            .roles
            .iter()
            .map(|role| role.role_id.as_str())
            .collect::<Vec<_>>(),
        [
            owner_role.id.to_string(),
            custom_role.id.to_string(),
            member_role.id.to_string()
        ]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
    );
    assert_eq!(summary.member_role_ids, vec![custom_role.id.to_string()]);
    assert_eq!(summary.roles[1].color, "#ef4444");
    assert!(
        serde_json::to_value(&summary)
            .unwrap()
            .get("user_role_ids")
            .is_none(),
        "summary must not send assignments for the entire membership"
    );

    let preview = super::super::support::server_summary(&state, &server, &outsider_id, false).await;
    assert!(preview.member_role_ids.is_empty());
}
