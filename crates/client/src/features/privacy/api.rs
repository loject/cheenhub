//! REST-клиент настроек приватности и пользовательских блокировок.

use cheenhub_contracts::rest::{BlockedUserSummary, ListBlockedUsersResponse, UserPrivacySettings};
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::features::auth::api as auth_api;

const NETWORK_ERROR_MESSAGE: &str = "Не удалось связаться с сервером.";

/// Загружает настройки входящих контактов текущего пользователя.
pub(crate) async fn get_settings() -> Result<UserPrivacySettings, String> {
    authorized_json("GET", "/privacy/settings", None).await
}

/// Сохраняет три независимых правила входящих контактов.
pub(crate) async fn update_settings(
    settings: UserPrivacySettings,
) -> Result<UserPrivacySettings, String> {
    authorized_json("PUT", "/privacy/settings", Some(&settings)).await
}

/// Возвращает активные блокировки текущего пользователя.
pub(crate) async fn list_blocked_users() -> Result<Vec<BlockedUserSummary>, String> {
    authorized_json::<ListBlockedUsersResponse>("GET", "/privacy/blocks", None)
        .await
        .map(|response| response.users)
}

/// Блокирует пользователя и прекращает прямые контакты с ним.
pub(crate) async fn block_user(user_id: &str) -> Result<(), String> {
    authorized_empty("PUT", &format!("/privacy/blocks/{user_id}")).await
}

/// Снимает блокировку без восстановления дружбы.
pub(crate) async fn unblock_user(user_id: &str) -> Result<(), String> {
    authorized_empty("DELETE", &format!("/privacy/blocks/{user_id}")).await
}

async fn authorized_empty(method: &str, path: &str) -> Result<(), String> {
    let response = authorized_response(method, path, None).await?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(auth_api::read_error(response).await)
    }
}

async fn authorized_json<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<&UserPrivacySettings>,
) -> Result<T, String> {
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
    body: Option<&UserPrivacySettings>,
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
    body: Option<&UserPrivacySettings>,
) -> Result<Response, String> {
    let mut request = match method {
        "GET" => auth_api::get(path),
        "PUT" => auth_api::put(path),
        "DELETE" => auth_api::delete(path),
        _ => return Err("Неподдерживаемый API-метод.".to_owned()),
    }
    .header("Authorization", format!("Bearer {access_token}"));
    if let Some(body) = body {
        request = request.json(body);
    }
    request
        .send()
        .await
        .map_err(|_| NETWORK_ERROR_MESSAGE.to_owned())
}
