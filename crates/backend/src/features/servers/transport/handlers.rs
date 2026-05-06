//! Server HTTP handlers.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::{
    AcceptServerInviteResponse, ApiError, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse,
    UpdateServerRoomRequest, UpdateServerRoomResponse,
};

use crate::features::servers::application;
use crate::features::servers::error::ServerError;
use crate::state::AppState;

/// Creates a server owned by the current user.
pub(crate) async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateServerRequest>,
) -> Result<Json<CreateServerResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::create(&state, token, request).await.map(Json)
}

/// Lists servers owned by the current user.
pub(crate) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListServersResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::list(&state, token).await.map(Json)
}

/// Creates an invite for a server owned by the current user.
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

/// Loads server invite information for the current user.
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

/// Accepts a server invite for the current user.
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

/// Leaves a server as the current user.
pub(crate) async fn leave(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
) -> Result<StatusCode, ServerError> {
    let token = bearer_token(&headers)?;
    application::leave(&state, token, server_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Lists rooms available on a server for the current user.
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

/// Creates a room on a server owned by the current user.
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

/// Updates a room on a server owned by the current user.
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

/// Deletes a room from a server owned by the current user.
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
