//! Клиент API сессий текущего пользователя.

use cheenhub_contracts::rest::{ActiveSession, ActiveSessionsResponse};
use reqwest::StatusCode;

use super::api::{delete, fresh_access_token, get, read_error, refresh_access_token};
use super::storage;

/// Загружает активные сессии текущего аутентифицированного пользователя.
pub(crate) async fn active_sessions() -> Result<Vec<ActiveSession>, String> {
    let access_token = fresh_access_token().await?;
    let response = send_get_sessions_request(&access_token).await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        let access_token = refresh_access_token().await?;
        let response = send_get_sessions_request(&access_token).await?;
        return parse_sessions_response(response).await;
    }

    parse_sessions_response(response).await
}

/// Отзывает одну активную сессию текущего аутентифицированного пользователя.
pub(crate) async fn revoke_session(session_id: &str) -> Result<(), String> {
    let access_token = fresh_access_token().await?;
    let response =
        send_delete_request(&access_token, &format!("/auth/sessions/{session_id}")).await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        let access_token = refresh_access_token().await?;
        let response =
            send_delete_request(&access_token, &format!("/auth/sessions/{session_id}")).await?;
        return parse_empty_response(response).await;
    }

    parse_empty_response(response).await
}

/// Отзывает все активные сессии текущего аутентифицированного пользователя.
pub(crate) async fn revoke_all_sessions() -> Result<(), String> {
    let access_token = fresh_access_token().await?;
    let response = send_delete_request(&access_token, "/auth/sessions").await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        let access_token = refresh_access_token().await?;
        let response = send_delete_request(&access_token, "/auth/sessions").await?;
        return clear_after_success(response).await;
    }

    clear_after_success(response).await
}

async fn send_get_sessions_request(access_token: &str) -> Result<reqwest::Response, String> {
    get("/auth/sessions")
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn send_delete_request(access_token: &str, path: &str) -> Result<reqwest::Response, String> {
    delete(path)
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn parse_sessions_response(
    response: reqwest::Response,
) -> Result<Vec<ActiveSession>, String> {
    if response.status().is_success() {
        return response
            .json::<ActiveSessionsResponse>()
            .await
            .map(|response| response.sessions)
            .map_err(|_| "Не удалось прочитать список сессий.".to_owned());
    }

    Err(read_error(response).await)
}

async fn clear_after_success(response: reqwest::Response) -> Result<(), String> {
    parse_empty_response(response).await?;
    storage::clear();
    Ok(())
}

async fn parse_empty_response(response: reqwest::Response) -> Result<(), String> {
    if response.status().is_success() {
        return Ok(());
    }

    Err(read_error(response).await)
}
