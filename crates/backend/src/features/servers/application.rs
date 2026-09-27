//! Потоки приложения серверов.

use cheenhub_contracts::rest::{
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse, ServerInviteSummary,
    ServerRoomKind, UpdateServerRoomRequest, UpdateServerRoomResponse,
};
use chrono::Utc;
use uuid::Uuid;

use crate::features::auth::application as auth_application;
use crate::features::servers::domain::ServerRoomWriteAccess;
use crate::features::servers::error::ServerError;
use crate::features::servers::validation;
use crate::state::AppState;

use self::support::{
    current_user_id, map_auth_error, parse_server_id, room_summary, room_write_allowed,
    server_for_member_or_owner, server_for_room_manager, server_summary, user_role_ids,
    validated_write_role_ids,
};

mod accept_invite;
mod invite_settings;
mod members_settings;
mod own_invites;
mod profile;
mod role_settings;
mod support;
mod voice_settings;

pub(crate) use accept_invite::accept_invite;
pub(crate) use invite_settings::{
    kick_server_invite_member, list_server_invites, revoke_server_invite,
};
pub(crate) use members_settings::{
    assign_server_member_role, kick_server_member, list_server_members, revoke_server_member_role,
};
pub(crate) use own_invites::{create_invite, delete_own_invite, list_own_invite_links};
pub(crate) use profile::{update, update_avatar};
pub(crate) use role_settings::{list_server_roles, save_server_roles};
pub(crate) use voice_settings::{get_voice_settings, update_voice_settings};

/// Создает сервер, принадлежащий текущему пользователю.
pub(crate) async fn create(
    state: &AppState,
    access_token: &str,
    request: CreateServerRequest,
) -> Result<CreateServerResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let owner_user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let _lifecycle = state
        .auth_store
        .lock_account_lifecycle(&owner_user_id)
        .await
        .map_err(ServerError::Internal)?;
    auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let valid = validation::create_server(request.name)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = state
        .server_store
        .insert_server(&owner_user_id, valid.name)
        .await
        .map_err(ServerError::Internal)?;
    state
        .server_store
        .insert_server_member(&server.id, &owner_user_id)
        .await
        .map_err(ServerError::Internal)?;
    state
        .server_store
        .insert_server_room(
            &server.id,
            "общий".to_owned(),
            ServerRoomKind::TextAndVoice,
            ServerRoomWriteAccess::all_members(),
        )
        .await
        .map_err(ServerError::Internal)?;

    Ok(CreateServerResponse {
        server: server_summary(state, &server, &owner_user_id, true).await,
    })
}

/// Возвращает серверы, доступные текущему пользователю.
pub(crate) async fn list(
    state: &AppState,
    access_token: &str,
) -> Result<ListServersResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let servers = state
        .server_store
        .list_servers(&user_id)
        .await
        .map_err(ServerError::Internal)?;

    let mut summaries = Vec::with_capacity(servers.len());
    for access in &servers {
        summaries.push(server_summary(state, &access.server, &user_id, access.is_member).await);
    }
    Ok(ListServersResponse { servers: summaries })
}

/// Загружает информацию о приглашении сервера для текущего пользователя.
pub(crate) async fn invite_info(
    state: &AppState,
    access_token: &str,
    code: String,
) -> Result<ServerInviteInfoResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let code = Uuid::parse_str(&code)
        .map_err(|_| ServerError::BadRequest("Приглашение не найдено.".to_owned()))?;
    let Some(invite) = state
        .server_store
        .find_server_invite(&code)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
    };

    if invite
        .expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now())
    {
        return Err(ServerError::BadRequest(
            "Срок действия приглашения истек.".to_owned(),
        ));
    }
    if invite.revoked_at.is_some() {
        return Err(ServerError::BadRequest("Приглашение отозвано.".to_owned()));
    }

    let Some(server) = state
        .server_store
        .find_server(&invite.server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };
    let uses = state
        .server_store
        .count_server_invite_uses(&invite.id)
        .await
        .map_err(ServerError::Internal)?;
    let is_member = server.owner_user_id == user_id
        || state
            .server_store
            .find_active_server_member(&server.id, &user_id)
            .await
            .map_err(ServerError::Internal)?
            .is_some();
    if !is_member
        && let Some(exclusion) = state
            .server_store
            .find_active_server_member_exclusion(&server.id, &user_id, Utc::now())
            .await
            .map_err(ServerError::Internal)?
    {
        tracing::debug!(
            server_id = %server.id,
            user_id = %user_id,
            excluded_until = %exclusion.expires_at,
            "blocked invite lookup for excluded server member"
        );
        return Err(ServerError::BadRequest(format!(
            "Ты временно исключен с сервера до {}.",
            exclusion.expires_at.to_rfc3339()
        )));
    }

    Ok(ServerInviteInfoResponse {
        invite: ServerInviteSummary {
            code: invite.id.to_string(),
            uses,
            max_uses: invite.max_uses,
            expires_at: invite.expires_at.map(|expires_at| expires_at.to_rfc3339()),
        },
        server: server_summary(state, &server, &user_id, is_member).await,
    })
}

/// Покидает сервер от имени текущего пользователя.
pub(crate) async fn leave(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<(), ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let server_id = Uuid::parse_str(&server_id)
        .map_err(|_| ServerError::BadRequest("Сервер не найден.".to_owned()))?;
    let Some(server) = state
        .server_store
        .find_server(&server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };

    if server.owner_user_id == user_id {
        return Err(ServerError::BadRequest(
            "Владелец сервера не может покинуть сервер.".to_owned(),
        ));
    }

    let active_member = state
        .server_store
        .find_active_server_member(&server.id, &user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_some();
    if !active_member {
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    }

    state
        .server_store
        .leave_server(&server.id, &user_id)
        .await
        .map_err(ServerError::Internal)
}

/// Возвращает комнаты, доступные на сервере для текущего пользователя.
pub(crate) async fn list_rooms(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<ListServerRoomsResponse, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let server = server_for_member_or_owner(state, &server_id, &user_id).await?;
    let rooms = state
        .server_store
        .list_server_rooms(&server.id)
        .await
        .map_err(ServerError::Internal)?;
    let is_owner = server.owner_user_id == user_id;
    let is_member = is_owner
        || state
            .server_store
            .find_active_server_member(&server.id, &user_id)
            .await
            .map_err(ServerError::Internal)?
            .is_some();
    let member_role_ids = user_role_ids(state, &server.id, &user_id).await?;

    Ok(ListServerRoomsResponse {
        rooms: rooms
            .iter()
            .map(|room| {
                let can_write = room_write_allowed(room, is_owner, is_member, &member_role_ids);
                room_summary(room, can_write)
            })
            .collect(),
    })
}

/// Создает комнату на сервере, которым текущий пользователь управляет как владелец или по праву `ManageRooms`.
pub(crate) async fn create_room(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: CreateServerRoomRequest,
) -> Result<CreateServerRoomResponse, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let valid = validation::server_room(request.name, request.write_access)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = server_for_room_manager(state, &server_id, &user_id).await?;
    let write_access = resolve_write_access(state, &server.id, valid.write_access).await?;
    let room = state
        .server_store
        .insert_server_room(&server.id, valid.name, request.kind, write_access)
        .await
        .map_err(ServerError::Internal)?;

    tracing::info!(
        server_id = %server.id,
        room_id = %room.id,
        write_access_mode = ?room.write_access.mode,
        write_role_count = room.write_access.role_ids.len(),
        "created server room"
    );

    Ok(CreateServerRoomResponse {
        room: room_summary(&room, true),
    })
}

/// Обновляет комнату на сервере, которым текущий пользователь управляет как владелец или по праву `ManageRooms`.
pub(crate) async fn update_room(
    state: &AppState,
    access_token: &str,
    server_id: String,
    room_id: String,
    request: UpdateServerRoomRequest,
) -> Result<UpdateServerRoomResponse, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let room_id = Uuid::parse_str(&room_id)
        .map_err(|_| ServerError::BadRequest("Комната не найдена.".to_owned()))?;
    let valid = validation::server_room(request.name, request.write_access)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = server_for_room_manager(state, &server_id, &user_id).await?;
    let write_access = resolve_write_access(state, &server.id, valid.write_access).await?;
    let Some(room) = state
        .server_store
        .update_server_room(&server.id, &room_id, valid.name, request.kind, write_access)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Комната не найдена.".to_owned()));
    };

    tracing::info!(
        server_id = %server.id,
        room_id = %room.id,
        write_access_mode = ?room.write_access.mode,
        write_role_count = room.write_access.role_ids.len(),
        "updated server room"
    );

    Ok(UpdateServerRoomResponse {
        room: room_summary(&room, true),
    })
}

/// Удаляет комнату с сервера, которым текущий пользователь управляет как владелец или по праву `ManageRooms`.
pub(crate) async fn delete_room(
    state: &AppState,
    access_token: &str,
    server_id: String,
    room_id: String,
) -> Result<(), ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let room_id = Uuid::parse_str(&room_id)
        .map_err(|_| ServerError::BadRequest("Комната не найдена.".to_owned()))?;
    let server = server_for_room_manager(state, &server_id, &user_id).await?;
    let Some(room) = state
        .server_store
        .find_server_room(&server.id, &room_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Комната не найдена.".to_owned()));
    };
    let room_count = state
        .server_store
        .count_server_rooms(&server.id)
        .await
        .map_err(ServerError::Internal)?;

    if room_count <= 1 {
        return Err(ServerError::BadRequest(
            "Нельзя удалить последнюю комнату сервера.".to_owned(),
        ));
    }

    state
        .server_store
        .delete_server_room(&server.id, &room.id)
        .await
        .map_err(ServerError::Internal)?;

    tracing::info!(
        server_id = %server.id,
        room_id = %room.id,
        "deleted server room"
    );

    Ok(())
}

/// Проверяет принадлежность ролей серверу и возвращает нормализованную настройку доступа к записи.
async fn resolve_write_access(
    state: &AppState,
    server_id: &Uuid,
    write_access: ServerRoomWriteAccess,
) -> Result<ServerRoomWriteAccess, ServerError> {
    let role_ids = validated_write_role_ids(state, server_id, &write_access.role_ids).await?;

    Ok(ServerRoomWriteAccess {
        mode: write_access.mode,
        role_ids,
    })
}

#[cfg(test)]
mod tests;

/// Проверяет владение серверами перед удалением аккаунта.
pub(crate) async fn user_owns_servers(state: &AppState, user_id: &Uuid) -> anyhow::Result<bool> {
    Ok(state
        .server_store
        .list_servers(user_id)
        .await?
        .iter()
        .any(|access| access.server.owner_user_id == *user_id))
}
