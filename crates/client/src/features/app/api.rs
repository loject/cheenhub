//! Клиент API аутентифицированного приложения.

use cheenhub_contracts::rest::{
    AcceptServerInviteResponse, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse, ServerRoomKind,
    ServerRoomSummary, ServerSummary, UpdateServerRoomRequest, UpdateServerRoomResponse,
};
use gloo_net::http::Request;

use crate::features::auth::api as auth_api;

/// Загружает серверы, доступные текущему пользователю.
pub(crate) async fn list_servers() -> Result<Vec<ServerSummary>, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::get(&auth_api::url("/servers"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<ListServersResponse>()
            .await
            .map(|response| response.servers)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Создает сервер, принадлежащий текущему пользователю.
pub(crate) async fn create_server(name: String) -> Result<ServerSummary, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::post(&auth_api::url("/servers"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&CreateServerRequest { name })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<CreateServerResponse>()
            .await
            .map(|response| response.server)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Создает приглашение для сервера, где текущий пользователь имеет нужное право.
pub(crate) async fn create_server_invite(
    server_id: String,
    max_uses: Option<u32>,
    expires_in_days: Option<u32>,
) -> Result<String, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::post(&auth_api::url(&format!("/servers/{server_id}/invites")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&CreateServerInviteRequest {
            max_uses,
            expires_in_days,
        })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<CreateServerInviteResponse>()
            .await
            .map(|response| response.code)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Загружает информацию о приглашении на сервер для текущего пользователя.
pub(crate) async fn load_server_invite(code: String) -> Result<ServerInviteInfoResponse, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::get(&auth_api::url(&format!("/servers/invites/{code}")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<ServerInviteInfoResponse>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Принимает приглашение на сервер для текущего пользователя.
pub(crate) async fn accept_server_invite(
    code: String,
) -> Result<AcceptServerInviteResponse, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::post(&auth_api::url(&format!("/servers/invites/{code}/accept")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<AcceptServerInviteResponse>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Покидает сервер, доступный текущему пользователю.
pub(crate) async fn leave_server(server_id: String) -> Result<(), String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::delete(&auth_api::url(&format!("/servers/{server_id}/membership")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return Ok(());
    }

    Err(auth_api::read_error(response).await)
}

/// Загружает комнаты, доступные на сервере.
pub(crate) async fn list_server_rooms(server_id: String) -> Result<Vec<ServerRoomSummary>, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::get(&auth_api::url(&format!("/servers/{server_id}/rooms")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<ListServerRoomsResponse>()
            .await
            .map(|response| response.rooms)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Создает комнату на сервере, принадлежащем текущему пользователю.
pub(crate) async fn create_server_room(
    server_id: String,
    name: String,
    kind: ServerRoomKind,
) -> Result<ServerRoomSummary, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::post(&auth_api::url(&format!("/servers/{server_id}/rooms")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&CreateServerRoomRequest { name, kind })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<CreateServerRoomResponse>()
            .await
            .map(|response| response.room)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Обновляет комнату на сервере, принадлежащем текущему пользователю.
pub(crate) async fn update_server_room(
    server_id: String,
    room_id: String,
    name: String,
    kind: ServerRoomKind,
) -> Result<ServerRoomSummary, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::put(&auth_api::url(&format!(
        "/servers/{server_id}/rooms/{room_id}"
    )))
    .header("Authorization", &format!("Bearer {access_token}"))
    .json(&UpdateServerRoomRequest { name, kind })
    .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
    .send()
    .await
    .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<UpdateServerRoomResponse>()
            .await
            .map(|response| response.room)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Удаляет комнату с сервера, принадлежащего текущему пользователю.
pub(crate) async fn delete_server_room(server_id: String, room_id: String) -> Result<(), String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::delete(&auth_api::url(&format!(
        "/servers/{server_id}/rooms/{room_id}"
    )))
    .header("Authorization", &format!("Bearer {access_token}"))
    .send()
    .await
    .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return Ok(());
    }

    Err(auth_api::read_error(response).await)
}
