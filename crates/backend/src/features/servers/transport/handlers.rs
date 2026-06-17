//! HTTP-обработчики серверов.

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::{
    AcceptServerInviteResponse, ApiError, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse,
    UpdateServerAvatarResponse, UpdateServerRequest, UpdateServerResponse, UpdateServerRoomRequest,
    UpdateServerRoomResponse,
};

use crate::features::servers::application;
use crate::features::servers::error::ServerError;
use crate::state::AppState;

/// Создает сервер, принадлежащий текущему пользователю.
pub(crate) async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateServerRequest>,
) -> Result<Json<CreateServerResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::create(&state, token, request).await.map(Json)
}

/// Возвращает серверы, принадлежащие текущему пользователю.
pub(crate) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListServersResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::list(&state, token).await.map(Json)
}

/// Обновляет сервер, принадлежащий текущему пользователю.
pub(crate) async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    Json(request): Json<UpdateServerRequest>,
) -> Result<Json<UpdateServerResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::update(&state, token, server_id, request)
        .await
        .map(Json)
}

/// Обновляет аватар сервера, принадлежащего текущему пользователю.
pub(crate) async fn update_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    bytes: Bytes,
) -> Result<Json<UpdateServerAvatarResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::update_avatar(&state, token, server_id, bytes)
        .await
        .map(Json)
}

/// Создает приглашение для сервера, принадлежащего текущему пользователю.
pub(crate) async fn create_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    Json(request): Json<CreateServerInviteRequest>,
) -> Result<Json<CreateServerInviteResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::create_invite(&state, token, server_id, request)
        .await
        .map(Json)
}

/// Загружает информацию о приглашении сервера для текущего пользователя.
pub(crate) async fn invite_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<ServerInviteInfoResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::invite_info(&state, token, code)
        .await
        .map(Json)
}

/// Принимает приглашение сервера для текущего пользователя.
pub(crate) async fn accept_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<AcceptServerInviteResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::accept_invite(&state, token, code)
        .await
        .map(Json)
}

/// Покидает сервер от имени текущего пользователя.
pub(crate) async fn leave(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
) -> Result<StatusCode, ServerError> {
    let token = bearer_token(&headers)?;
    application::leave(&state, token, server_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Возвращает комнаты, доступные на сервере для текущего пользователя.
pub(crate) async fn list_rooms(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
) -> Result<Json<ListServerRoomsResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::list_rooms(&state, token, server_id)
        .await
        .map(Json)
}

/// Создает комнату на сервере, принадлежащем текущему пользователю.
pub(crate) async fn create_room(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    Json(request): Json<CreateServerRoomRequest>,
) -> Result<Json<CreateServerRoomResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::create_room(&state, token, server_id, request)
        .await
        .map(Json)
}

/// Обновляет комнату на сервере, принадлежащем текущему пользователю.
pub(crate) async fn update_room(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, room_id)): Path<(String, String)>,
    Json(request): Json<UpdateServerRoomRequest>,
) -> Result<Json<UpdateServerRoomResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::update_room(&state, token, server_id, room_id, request)
        .await
        .map(Json)
}

/// Удаляет комнату с сервера, принадлежащего текущему пользователю.
pub(crate) async fn delete_room(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server_id, room_id)): Path<(String, String)>,
) -> Result<StatusCode, ServerError> {
    let token = bearer_token(&headers)?;
    application::delete_room(&state, token, server_id, room_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::Internal(error) => {
                tracing::error!(%error, "server request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Не удалось выполнить запрос. Попробуй еще раз.".to_owned(),
                )
            }
        };

        (
            status,
            Json(ApiError {
                code: code.to_owned(),
                message,
            }),
        )
            .into_response()
    }
}

// TODO: метод встречается в нескольких местах
fn bearer_token(headers: &HeaderMap) -> Result<&str, ServerError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ServerError::Unauthorized("Войди, чтобы продолжить.".to_owned()))?;

    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ServerError::Unauthorized("Войди, чтобы продолжить.".to_owned()))
}
