//! Клиент API профиля текущего пользователя.

use cheenhub_contracts::rest::{
    AuthUser, ChangeCurrentUserPasswordRequest, UpdateCurrentUserRequest,
};
use reqwest::StatusCode;

use super::api::{fresh_access_token, patch, post, put, read_error, refresh_access_token};

/// Обновляет профиль текущего аутентифицированного пользователя.
pub(crate) async fn update_current_user(
    request: UpdateCurrentUserRequest,
) -> Result<AuthUser, String> {
    let access_token = fresh_access_token().await?;
    let response = send_update_request(&access_token, &request).await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        let access_token = refresh_access_token().await?;
        let response = send_update_request(&access_token, &request).await?;
        return parse_user_response(response).await;
    }

    parse_user_response(response).await
}

/// Меняет пароль текущего аутентифицированного пользователя.
pub(crate) async fn change_current_user_password(
    request: ChangeCurrentUserPasswordRequest,
) -> Result<(), String> {
    let access_token = fresh_access_token().await?;
    let response = send_password_change_request(&access_token, &request).await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        let access_token = refresh_access_token().await?;
        let response = send_password_change_request(&access_token, &request).await?;
        return parse_empty_response(response).await;
    }

    parse_empty_response(response).await
}

/// Загружает и заменяет аватар текущего аутентифицированного пользователя.
pub(crate) async fn update_current_user_avatar(bytes: Vec<u8>) -> Result<AuthUser, String> {
    let access_token = fresh_access_token().await?;
    let response = send_avatar_update_request(&access_token, &bytes).await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        let access_token = refresh_access_token().await?;
        let response = send_avatar_update_request(&access_token, &bytes).await?;
        return parse_user_response(response).await;
    }

    parse_user_response(response).await
}

async fn send_update_request(
    access_token: &str,
    request: &UpdateCurrentUserRequest,
) -> Result<reqwest::Response, String> {
    patch("/auth/me")
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(request)
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn send_password_change_request(
    access_token: &str,
    request: &ChangeCurrentUserPasswordRequest,
) -> Result<reqwest::Response, String> {
    post("/auth/me/password")
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(request)
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn send_avatar_update_request(
    access_token: &str,
    bytes: &[u8],
) -> Result<reqwest::Response, String> {
    put("/auth/me/avatar")
        .header("Authorization", &format!("Bearer {access_token}"))
        .header("Content-Type", "application/octet-stream")
        .body(bytes.to_vec())
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn parse_user_response(response: reqwest::Response) -> Result<AuthUser, String> {
    if response.status().is_success() {
        return response
            .json::<AuthUser>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(read_error(response).await)
}

async fn parse_empty_response(response: reqwest::Response) -> Result<(), String> {
    if response.status().is_success() {
        return Ok(());
    }

    Err(read_error(response).await)
}
