//! Shared server application helpers.

use cheenhub_contracts::rest::{ServerRoomSummary, ServerSummary};
use uuid::Uuid;

use crate::features::auth::application as auth_application;
use crate::features::auth::error::AuthError;
use crate::features::servers::domain::{Server, ServerRoom};
use crate::features::servers::error::ServerError;
use crate::state::AppState;

pub(super) fn server_summary(server: &Server, user_id: &Uuid, is_member: bool) -> ServerSummary {
    ServerSummary {
        id: server.id.to_string(),
        name: server.name.clone(),
        is_owner: server.owner_user_id == *user_id,
        is_member,
    }
}

pub(super) fn room_summary(room: &ServerRoom) -> ServerRoomSummary {
    ServerRoomSummary {
        id: room.id.to_string(),
        name: room.name.clone(),
        kind: room.kind,
        position: room.position,
    }
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

pub(super) fn map_auth_error(error: AuthError) -> ServerError {
    match error {
        AuthError::BadRequest(message) | AuthError::Unauthorized(message) => {
            ServerError::Unauthorized(message)
        }
        AuthError::Conflict(message) => ServerError::BadRequest(message),
        AuthError::Misconfigured { message, .. } => ServerError::Internal(anyhow::anyhow!(message)),
        AuthError::Internal(error) => ServerError::Internal(error),
    }
}
