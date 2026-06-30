//! HTTP-обработчики друзей и личных сообщений.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::{
    ApiError, ListDmConversationsResponse, ListDmMessagesResponse, ListFriendRequestsResponse,
    ListFriendsResponse, MarkDmConversationReadRequest, MarkDmConversationReadResponse,
    OpenDmConversationRequest, OpenDmConversationResponse, SearchUsersResponse,
    SendDmMessageRequest, SendDmMessageResponse, SendFriendRequestRequest,
    SendFriendRequestResponse,
};
use serde::Deserialize;

use crate::features::social::application;
use crate::features::social::error::SocialError;
use crate::state::AppState;

/// Query-параметры поиска пользователей.
#[derive(Deserialize)]
pub(crate) struct SearchUsersQuery {
    /// Строка поиска по никнейму.
    q: Option<String>,
}

/// Query-параметры истории личного диалога.
#[derive(Deserialize)]
pub(crate) struct DmMessagesQuery {
    /// Сообщение-курсор, перед которым нужно вернуть более старую страницу.
    before_message_id: Option<String>,
}

/// Ищет пользователей по никнейму.
pub(crate) async fn search_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<SearchUsersResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::search_users(&state, token, query.q)
        .await
        .map(Json)
}

/// Возвращает друзей текущего пользователя.
pub(crate) async fn list_friends(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListFriendsResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::list_friends(&state, token).await.map(Json)
}

/// Возвращает входящие заявки.
pub(crate) async fn list_incoming_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListFriendRequestsResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::list_incoming_requests(&state, token)
        .await
        .map(Json)
}

/// Возвращает исходящие заявки.
pub(crate) async fn list_outgoing_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListFriendRequestsResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::list_outgoing_requests(&state, token)
        .await
        .map(Json)
}

/// Отправляет заявку в друзья.
pub(crate) async fn send_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SendFriendRequestRequest>,
) -> Result<Json<SendFriendRequestResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::send_friend_request(&state, token, request)
        .await
        .map(Json)
}

/// Принимает входящую заявку.
pub(crate) async fn accept_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<Json<SendFriendRequestResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::accept_friend_request(&state, token, request_id)
        .await
        .map(Json)
}

/// Отклоняет входящую заявку.
pub(crate) async fn decline_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<Json<SendFriendRequestResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::decline_friend_request(&state, token, request_id)
        .await
        .map(Json)
}

/// Отменяет исходящую заявку.
pub(crate) async fn cancel_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<Json<SendFriendRequestResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::cancel_friend_request(&state, token, request_id)
        .await
        .map(Json)
}

/// Удаляет друга.
pub(crate) async fn delete_friend(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(friend_user_id): Path<String>,
) -> Result<StatusCode, SocialError> {
    let token = bearer_token(&headers)?;
    application::delete_friend(&state, token, friend_user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Возвращает диалоги личных сообщений.
pub(crate) async fn list_dm_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListDmConversationsResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::list_dm_conversations(&state, token)
        .await
        .map(Json)
}

/// Открывает личный диалог с другом.
pub(crate) async fn open_dm_conversation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OpenDmConversationRequest>,
) -> Result<Json<OpenDmConversationResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::open_dm_conversation(&state, token, request)
        .await
        .map(Json)
}

/// Возвращает страницу сообщений личного диалога.
pub(crate) async fn list_dm_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Query(query): Query<DmMessagesQuery>,
) -> Result<Json<ListDmMessagesResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::list_dm_messages(&state, token, conversation_id, query.before_message_id)
        .await
        .map(Json)
}

/// Отправляет личное сообщение.
pub(crate) async fn send_dm_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Json(request): Json<SendDmMessageRequest>,
) -> Result<Json<SendDmMessageResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::send_dm_message(&state, token, conversation_id, request)
        .await
        .map(Json)
}

/// Помечает личный диалог прочитанным до указанного сообщения.
pub(crate) async fn mark_dm_conversation_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Json(request): Json<MarkDmConversationReadRequest>,
) -> Result<Json<MarkDmConversationReadResponse>, SocialError> {
    let token = bearer_token(&headers)?;
    application::mark_dm_conversation_read(&state, token, conversation_id, request)
        .await
        .map(Json)
}

impl IntoResponse for SocialError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            Self::Internal(error) => {
                tracing::error!(%error, "social request failed");
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

fn bearer_token(headers: &HeaderMap) -> Result<&str, SocialError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| SocialError::Unauthorized("Войди, чтобы продолжить.".to_owned()))?;

    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| SocialError::Unauthorized("Войди, чтобы продолжить.".to_owned()))
}
