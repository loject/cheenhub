//! Text chat event delivery policy.

use cheenhub_contracts::realtime::ServerRoleKind;
use cheenhub_contracts::realtime::ServerRolePermission;
use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use crate::state::AppState;

/// Returns whether a user can delete any message in the server (owner or has DeleteMessages role).
pub(crate) async fn can_delete_any_message(
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
        role.kind != ServerRoleKind::Member
            && user_role_ids.contains(&role.id)
            && role
                .permissions
                .contains(&ServerRolePermission::DeleteMessages)
    }))
}

/// Returns whether a user can receive room-scoped text chat events.
pub(crate) async fn can_receive_room_event(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
    room_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(room) = state
        .server_store
        .find_server_room(server_id, room_id)
        .await?
    else {
        return Ok(false);
    };
    if room.kind == ServerRoomKind::Voice {
        return Ok(false);
    }

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
