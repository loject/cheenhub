//! Server settings REST client helpers.

use cheenhub_contracts::rest::{
    ServerSummary, UpdateServerAvatarResponse, UpdateServerRequest, UpdateServerResponse,
};
use gloo_net::http::Request;

use crate::features::auth::api as auth_api;

/// Updates a server profile.
pub(super) async fn update_server(
    server_id: String,
    request: UpdateServerRequest,
) -> Result<ServerSummary, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::put(&auth_api::url(&format!("/servers/{server_id}")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&request)
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<UpdateServerResponse>()
            .await
            .map(|response| response.server)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Uploads and replaces a server avatar.
pub(super) async fn update_server_avatar(
    server_id: String,
    bytes: Vec<u8>,
) -> Result<ServerSummary, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::put(&auth_api::url(&format!("/servers/{server_id}/avatar")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<UpdateServerAvatarResponse>()
            .await
            .map(|response| response.server)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}
