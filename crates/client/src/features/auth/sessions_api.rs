//! Клиент API сессий текущего пользователя.

use cheenhub_contracts::rest::{ActiveSession, ActiveSessionsResponse};
use gloo_net::http::Request;

use super::api::{fresh_access_token, read_error, refresh_access_token, url};
use super::storage;

/// Загружает активные сессии текущего аутентифицированного пользователя.
pub(crate) async fn active_sessions() -> Result<Vec<ActiveSession>, String> {
    let access_token = fresh_access_token().await?;
    let response = send_get_sessions_request(&access_token).await?;

    if response.status() == 401 {
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

    if response.status() == 401 {
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

    if response.status() == 401 {
        let access_token = refresh_access_token().await?;
        let response = send_delete_request(&access_token, "/auth/sessions").await?;
        return clear_after_success(response).await;
    }

    clear_after_success(response).await
}

async fn send_get_sessions_request(access_token: &str) -> Result<gloo_net::http::Response, String> {
    Request::get(&url("/auth/sessions"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn send_delete_request(
    access_token: &str,
    path: &str,
) -> Result<gloo_net::http::Response, String> {
    Request::delete(&url(path))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn parse_sessions_response(
    response: gloo_net::http::Response,
) -> Result<Vec<ActiveSession>, String> {
    if response.ok() {
        return response
            .json::<ActiveSessionsResponse>()
            .await
            .map(|response| response.sessions)
            .map_err(|_| "Не удалось прочитать список сессий.".to_owned());
    }

    Err(read_error(response).await)
}

async fn clear_after_success(response: gloo_net::http::Response) -> Result<(), String> {
    parse_empty_response(response).await?;
    storage::clear();
    Ok(())
}

async fn parse_empty_response(response: gloo_net::http::Response) -> Result<(), String> {
    if response.ok() {
        return Ok(());
    }

    Err(read_error(response).await)
}
