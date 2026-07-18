//! REST API клиента для друзей и личных сообщений.

use cheenhub_contracts::rest::{
    DmConversationSummary, DmImageAttachmentSummary, DmMessageSummary, ListDmConversationsResponse,
    ListDmMessagesResponse, ListFriendRequestsResponse, ListFriendsResponse,
    MarkDmConversationReadRequest, MarkDmConversationReadResponse, OpenDmConversationRequest,
    OpenDmConversationResponse, SearchUsersResponse, SendDmMessageRequest, SendDmMessageResponse,
    SendFriendRequestRequest, SendFriendRequestResponse, UploadDmImageResponse, UserSearchResult,
};
use reqwest::{Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::features::auth::api as auth_api;

const NETWORK_ERROR_MESSAGE: &str = "Не удалось связаться с сервером.";

/// Загружает друзей текущего пользователя.
pub(crate) async fn list_friends() -> Result<Vec<cheenhub_contracts::rest::FriendSummary>, String> {
    authorized_json::<ListFriendsResponse>("GET", "/friends", None::<&()>)
        .await
        .map(|response| response.friends)
}

/// Загружает входящие заявки.
pub(crate) async fn list_incoming_requests()
-> Result<Vec<cheenhub_contracts::rest::FriendRequestSummary>, String> {
    authorized_json::<ListFriendRequestsResponse>("GET", "/friends/requests/incoming", None::<&()>)
        .await
        .map(|response| response.requests)
}

/// Загружает исходящие заявки.
pub(crate) async fn list_outgoing_requests()
-> Result<Vec<cheenhub_contracts::rest::FriendRequestSummary>, String> {
    authorized_json::<ListFriendRequestsResponse>("GET", "/friends/requests/outgoing", None::<&()>)
        .await
        .map(|response| response.requests)
}

/// Ищет пользователей по никнейму.
pub(crate) async fn search_users(query: &str) -> Result<Vec<UserSearchResult>, String> {
    let path = format!("/friends/search?q={}", encode_query(query));
    authorized_json::<SearchUsersResponse>("GET", &path, None::<&()>)
        .await
        .map(|response| response.users)
}

fn encode_query(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![char::from(byte)]
            }
            b' ' => vec!['+'],
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

/// Отправляет заявку в друзья.
pub(crate) async fn send_friend_request(
    recipient_user_id: String,
) -> Result<SendFriendRequestResponse, String> {
    authorized_json(
        "POST",
        "/friends/requests",
        Some(&SendFriendRequestRequest { recipient_user_id }),
    )
    .await
}

/// Принимает входящую заявку.
pub(crate) async fn accept_friend_request(
    request_id: &str,
) -> Result<SendFriendRequestResponse, String> {
    authorized_json(
        "POST",
        &format!("/friends/requests/{request_id}/accept"),
        None::<&()>,
    )
    .await
}

/// Отклоняет входящую заявку.
pub(crate) async fn decline_friend_request(
    request_id: &str,
) -> Result<SendFriendRequestResponse, String> {
    authorized_json(
        "POST",
        &format!("/friends/requests/{request_id}/decline"),
        None::<&()>,
    )
    .await
}

/// Отменяет исходящую заявку.
pub(crate) async fn cancel_friend_request(
    request_id: &str,
) -> Result<SendFriendRequestResponse, String> {
    authorized_json(
        "POST",
        &format!("/friends/requests/{request_id}/cancel"),
        None::<&()>,
    )
    .await
}

/// Удаляет друга.
pub(crate) async fn delete_friend(friend_user_id: &str) -> Result<(), String> {
    let response =
        authorized_response("DELETE", &format!("/friends/{friend_user_id}"), None::<&()>).await?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(auth_api::read_error(response).await)
    }
}

/// Загружает личные диалоги.
pub(crate) async fn list_dm_conversations() -> Result<Vec<DmConversationSummary>, String> {
    authorized_json::<ListDmConversationsResponse>(
        "GET",
        "/direct-messages/conversations",
        None::<&()>,
    )
    .await
    .map(|response| response.conversations)
}

/// Открывает личный диалог с другом.
pub(crate) async fn open_dm_conversation(
    friend_user_id: String,
) -> Result<DmConversationSummary, String> {
    authorized_json::<OpenDmConversationResponse>(
        "POST",
        "/direct-messages/conversations",
        Some(&OpenDmConversationRequest { friend_user_id }),
    )
    .await
    .map(|response| response.conversation)
}

/// Загружает сообщения личного диалога.
pub(crate) async fn list_dm_messages(
    conversation_id: &str,
    before_message_id: Option<&str>,
) -> Result<ListDmMessagesResponse, String> {
    let query = before_message_id
        .map(|message_id| format!("?before_message_id={message_id}"))
        .unwrap_or_default();
    authorized_json::<ListDmMessagesResponse>(
        "GET",
        &format!("/direct-messages/conversations/{conversation_id}/messages{query}"),
        None::<&()>,
    )
    .await
}

/// Помечает личный диалог прочитанным до указанного сообщения.
pub(crate) async fn mark_dm_conversation_read(
    conversation_id: &str,
    last_read_message_id: String,
) -> Result<MarkDmConversationReadResponse, String> {
    authorized_json::<MarkDmConversationReadResponse>(
        "POST",
        &format!("/direct-messages/conversations/{conversation_id}/read"),
        Some(&MarkDmConversationReadRequest {
            last_read_message_id,
        }),
    )
    .await
}

/// Отправляет личное сообщение.
pub(crate) async fn send_dm_message(
    conversation_id: &str,
    body: String,
    image_id: Option<String>,
) -> Result<DmMessageSummary, String> {
    authorized_json::<SendDmMessageResponse>(
        "POST",
        &format!("/direct-messages/conversations/{conversation_id}/messages"),
        Some(&SendDmMessageRequest { body, image_id }),
    )
    .await
    .map(|response| response.message)
}

/// Загружает изображение для личного сообщения.
pub(crate) async fn upload_dm_image(
    conversation_id: &str,
    bytes: Vec<u8>,
) -> Result<DmImageAttachmentSummary, String> {
    let path = format!("/direct-messages/conversations/{conversation_id}/images");
    let response = authorized_bytes_response("POST", &path, Some(bytes)).await?;
    if response.status().is_success() {
        response
            .json::<UploadDmImageResponse>()
            .await
            .map(|value| value.image)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())
    } else {
        Err(auth_api::read_error(response).await)
    }
}

/// Загружает байты изображения личного сообщения.
pub(crate) async fn load_dm_image(
    conversation_id: &str,
    image_id: &str,
) -> Result<Vec<u8>, String> {
    let path = format!("/direct-messages/conversations/{conversation_id}/images/{image_id}");
    let response = authorized_bytes_response("GET", &path, None).await?;
    if response.status().is_success() {
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|_| "Не удалось загрузить изображение.".to_owned())
    } else {
        Err(auth_api::read_error(response).await)
    }
}

async fn authorized_bytes_response(
    method: &str,
    path: &str,
    body: Option<Vec<u8>>,
) -> Result<Response, String> {
    let token = auth_api::fresh_access_token().await?;
    let response = send_bytes(method, path, &token, body.clone()).await?;
    if response.status() != StatusCode::UNAUTHORIZED {
        return Ok(response);
    }
    let token = auth_api::refresh_access_token().await?;
    send_bytes(method, path, &token, body).await
}

async fn send_bytes(
    method: &str,
    path: &str,
    token: &str,
    body: Option<Vec<u8>>,
) -> Result<Response, String> {
    let request = match method {
        "GET" => auth_api::get(path),
        _ => auth_api::post(path),
    }
    .header("Authorization", format!("Bearer {token}"));
    request
        .body(body.unwrap_or_default())
        .send()
        .await
        .map_err(|_| NETWORK_ERROR_MESSAGE.to_owned())
}

async fn authorized_json<T>(
    method: &str,
    path: &str,
    body: Option<&impl Serialize>,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = authorized_response(method, path, body).await?;
    if response.status().is_success() {
        response
            .json::<T>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())
    } else {
        Err(auth_api::read_error(response).await)
    }
}

async fn authorized_response(
    method: &str,
    path: &str,
    body: Option<&impl Serialize>,
) -> Result<Response, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = send(method, path, &access_token, body).await?;
    if response.status() != StatusCode::UNAUTHORIZED {
        return Ok(response);
    }

    let access_token = auth_api::refresh_access_token().await?;
    send(method, path, &access_token, body).await
}

async fn send(
    method: &str,
    path: &str,
    access_token: &str,
    body: Option<&impl Serialize>,
) -> Result<Response, String> {
    let mut request = match method {
        "GET" => auth_api::get(path),
        "POST" => auth_api::post(path),
        "DELETE" => auth_api::delete(path),
        _ => return Err("Неподдерживаемый API-метод.".to_owned()),
    }
    .header("Authorization", &format!("Bearer {access_token}"));
    if let Some(body) = body {
        request = request.json(body);
    }
    request
        .send()
        .await
        .map_err(|_| NETWORK_ERROR_MESSAGE.to_owned())
}
