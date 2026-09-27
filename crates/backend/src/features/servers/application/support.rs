//! Shared server application helpers.

use cheenhub_contracts::realtime::{ServerRoleKind, ServerRolePermission, ServerRoleSummary};
use cheenhub_contracts::rest::{
    ServerRoomSummary, ServerRoomWriteAccess, ServerRoomWriteAccessMode, ServerSummary,
};
use uuid::Uuid;

use crate::features::auth::application as auth_application;
use crate::features::auth::error::AuthError;
use crate::features::images::application as image_application;
use crate::features::servers::domain::{Server, ServerRoom};
use crate::features::servers::error::ServerError;
use crate::state::AppState;

pub(super) async fn server_summary(
    state: &AppState,
    server: &Server,
    user_id: &Uuid,
    is_member: bool,
) -> ServerSummary {
    let is_owner = server.owner_user_id == *user_id;
    let (roles, member_role_ids) = fetch_role_data(state, &server.id, user_id).await;
    ServerSummary {
        id: server.id.to_string(),
        name: server.name.clone(),
        avatar_url: server
            .avatar_image_id
            .map(|image_id| image_application::avatar_url(state, &image_id)),
        is_owner,
        is_member,
        roles,
        member_role_ids,
    }
}

async fn fetch_role_data(
    state: &AppState,
    server_id: &Uuid,
    user_id: &Uuid,
) -> (Vec<ServerRoleSummary>, Vec<String>) {
    let roles = match state.server_store.list_server_roles(server_id).await {
        Ok(roles) => roles
            .into_iter()
            .map(|role| ServerRoleSummary {
                role_id: role.id.to_string(),
                name: role.name,
                color: role.color,
                kind: role.kind,
                permissions: role.permissions,
            })
            .filter(|role| role.kind != ServerRoleKind::Owner)
            .collect(),
        Err(_) => Vec::new(),
    };
    let member_role_ids = match state.server_store.list_server_member_roles(server_id).await {
        Ok(pairs) => pairs
            .into_iter()
            .filter(|(uid, _)| uid == user_id)
            .map(|(_, rid)| rid.to_string())
            .collect(),
        Err(_) => Vec::new(),
    };
    (roles, member_role_ids)
}

pub(super) fn room_summary(room: &ServerRoom, can_write: bool) -> ServerRoomSummary {
    ServerRoomSummary {
        id: room.id.to_string(),
        name: room.name.clone(),
        kind: room.kind,
        position: room.position,
        write_access: ServerRoomWriteAccess {
            mode: room.write_access.mode,
            role_ids: room
                .write_access
                .role_ids
                .iter()
                .map(|role_id| role_id.to_string())
                .collect(),
        },
        can_write,
    }
}

/// Определяет, может ли пользователь писать в комнату по настройке доступа и своим ролям.
pub(super) fn room_write_allowed(
    room: &ServerRoom,
    is_owner: bool,
    is_member: bool,
    member_role_ids: &[Uuid],
) -> bool {
    if is_owner {
        return true;
    }
    if !is_member {
        return false;
    }
    if room.write_access.mode == ServerRoomWriteAccessMode::AllMembers {
        return true;
    }

    room.write_access
        .role_ids
        .iter()
        .any(|role_id| member_role_ids.contains(role_id))
}

/// Возвращает идентификаторы ролей, назначенных пользователю на сервере.
pub(super) async fn user_role_ids(
    state: &AppState,
    server_id: &Uuid,
    user_id: &Uuid,
) -> Result<Vec<Uuid>, ServerError> {
    Ok(state
        .server_store
        .list_server_member_roles(server_id)
        .await
        .map_err(ServerError::Internal)?
        .into_iter()
        .filter(|(member_user_id, _)| member_user_id == user_id)
        .map(|(_, role_id)| role_id)
        .collect())
}

/// Проверяет, что все переданные роли принадлежат серверу, и возвращает их идентификаторы.
pub(super) async fn validated_write_role_ids(
    state: &AppState,
    server_id: &Uuid,
    role_ids: &[Uuid],
) -> Result<Vec<Uuid>, ServerError> {
    if role_ids.is_empty() {
        return Ok(Vec::new());
    }

    let server_roles = state
        .server_store
        .list_server_roles(server_id)
        .await
        .map_err(ServerError::Internal)?;
    let mut result = Vec::new();
    for role_id in role_ids {
        if !server_roles.iter().any(|role| role.id == *role_id) {
            return Err(ServerError::BadRequest(
                "Выбрана роль, которая не принадлежит этому серверу.".to_owned(),
            ));
        }
        result.push(*role_id);
    }

    Ok(result)
}

pub(super) async fn current_user_id(
    state: &AppState,
    access_token: &str,
) -> Result<Uuid, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;

    Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))
}

pub(super) fn parse_server_id(server_id: String) -> Result<Uuid, ServerError> {
    Uuid::parse_str(&server_id).map_err(|_| ServerError::BadRequest("Сервер не найден.".to_owned()))
}

pub(super) async fn owned_server(
    state: &AppState,
    server_id: &Uuid,
    owner_user_id: &Uuid,
) -> Result<Server, ServerError> {
    state
        .server_store
        .find_owned_server(server_id, owner_user_id)
        .await
        .map_err(ServerError::Internal)?
        .ok_or_else(|| ServerError::NotFound("Сервер не найден или недоступен.".to_owned()))
}

pub(super) async fn server_for_member_or_owner(
    state: &AppState,
    server_id: &Uuid,
    user_id: &Uuid,
) -> Result<Server, ServerError> {
    let Some(server) = state
        .server_store
        .find_server(server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };

    if server.owner_user_id == *user_id {
        return Ok(server);
    }

    let is_member = state
        .server_store
        .find_active_server_member(&server.id, user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_some();
    if is_member {
        return Ok(server);
    }

    Err(ServerError::NotFound(
        "Сервер не найден или недоступен.".to_owned(),
    ))
}

pub(super) async fn user_has_server_permission(
    state: &AppState,
    server: &Server,
    user_id: &Uuid,
    permission: ServerRolePermission,
) -> Result<bool, ServerError> {
    if server.owner_user_id == *user_id {
        return Ok(true);
    }

    if state
        .server_store
        .find_active_server_member(&server.id, user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_none()
    {
        return Ok(false);
    }

    let roles = state
        .server_store
        .list_server_roles(&server.id)
        .await
        .map_err(ServerError::Internal)?;
    let member_role_ids = user_role_ids(state, &server.id, user_id).await?;

    Ok(roles.iter().any(|role| {
        (role.kind == ServerRoleKind::Member || member_role_ids.contains(&role.id))
            && role.permissions.contains(&permission)
    }))
}

/// Возвращает сервер, если текущий пользователь владеет им или имеет право `ManageRooms`.
pub(super) async fn server_for_room_manager(
    state: &AppState,
    server_id: &Uuid,
    user_id: &Uuid,
) -> Result<Server, ServerError> {
    let server = server_for_member_or_owner(state, server_id, user_id).await?;
    if user_has_server_permission(state, &server, user_id, ServerRolePermission::ManageRooms)
        .await?
    {
        return Ok(server);
    }

    Err(ServerError::NotFound(
        "Сервер не найден или недоступен.".to_owned(),
    ))
}

pub(super) fn map_auth_error(error: AuthError) -> ServerError {
    match error {
        AuthError::BadRequest(message) | AuthError::Unauthorized(message) => {
            ServerError::Unauthorized(message)
        }
        AuthError::RefreshRejected { message, .. }
        | AuthError::RefreshRotationInProgress(message) => ServerError::Unauthorized(message),
        AuthError::Conflict(message) | AuthError::RateLimited(message) => {
            ServerError::BadRequest(message)
        }
        AuthError::Misconfigured { message, .. } => ServerError::Internal(anyhow::anyhow!(message)),
        AuthError::Internal(error) => ServerError::Internal(error),
    }
}
