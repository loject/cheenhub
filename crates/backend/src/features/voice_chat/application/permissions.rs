//! Проверки доступа и прав ролей для действий голосовых комнат.

use cheenhub_contracts::realtime::{ServerRoleKind, ServerRolePermission};
use uuid::Uuid;

use crate::state::AppState;

/// Возвращает, может ли пользователь кикать участников голосовых комнат сервера.
pub(super) async fn user_can_kick_voice(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(server) = state.server_store.find_server(server_id).await? else {
        return Ok(false);
    };
    if server.owner_user_id == *user_id {
        return Ok(true);
    }
    if state
        .server_store
        .find_active_server_member(server_id, user_id)
        .await?
        .is_none()
    {
        return Ok(false);
    }

    let roles = state.server_store.list_server_roles(server_id).await?;
    let member_roles = state
        .server_store
        .list_server_member_roles(server_id)
        .await?;
    let user_role_ids: Vec<_> = member_roles
        .iter()
        .filter(|(uid, _)| uid == user_id)
        .map(|(_, rid)| *rid)
        .collect();

    Ok(roles.iter().any(|role| {
        (role.kind == ServerRoleKind::Member || user_role_ids.contains(&role.id))
            && role
                .permissions
                .contains(&ServerRolePermission::KickVoiceMembers)
    }))
}

/// Возвращает, есть ли у пользователя доступ к серверу.
pub(super) async fn user_has_server_access(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(server) = state.server_store.find_server(server_id).await? else {
        return Ok(false);
    };
    if server.owner_user_id == *user_id {
        return Ok(true);
    }

    Ok(state
        .server_store
        .find_active_server_member(server_id, user_id)
        .await?
        .is_some())
}
