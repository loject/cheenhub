//! Current user profile API client.

use cheenhub_contracts::rest::{AuthUser, UpdateCurrentUserRequest};
use gloo_net::http::Request;

use super::api::{fresh_access_token, read_error, refresh_access_token, url};

/// Updates the current authenticated user profile.
pub(crate) async fn update_current_user(
    request: UpdateCurrentUserRequest,
) -> Result<AuthUser, String> {
    let access_token = fresh_access_token().await?;
    let response = send_update_request(&access_token, &request).await?;

    if response.status() == 401 {
        let access_token = refresh_access_token().await?;
        let response = send_update_request(&access_token, &request).await?;
        return parse_user_response(response).await;
    }

    parse_user_response(response).await
}

async fn send_update_request(
    access_token: &str,
    request: &UpdateCurrentUserRequest,
) -> Result<gloo_net::http::Response, String> {
    Request::patch(&url("/auth/me"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(request)
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn parse_user_response(response: gloo_net::http::Response) -> Result<AuthUser, String> {
    if response.ok() {
        return response
            .json::<AuthUser>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(read_error(response).await)
}
