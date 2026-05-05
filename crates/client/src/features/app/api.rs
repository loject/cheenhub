//! Authenticated app API client.

use cheenhub_contracts::rest::{
    CreateServerInviteRequest, CreateServerInviteResponse, CreateServerRequest,
    CreateServerResponse, ListServersResponse, ServerSummary,
};
use gloo_net::http::Request;

use crate::features::auth::api as auth_api;

/// Loads servers available to the current user.
pub(crate) async fn list_servers() -> Result<Vec<ServerSummary>, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::get(&auth_api::url("/servers"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<ListServersResponse>()
            .await
            .map(|response| response.servers)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Creates a server owned by the current user.
pub(crate) async fn create_server(name: String) -> Result<ServerSummary, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::post(&auth_api::url("/servers"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&CreateServerRequest { name })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<CreateServerResponse>()
            .await
            .map(|response| response.server)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Creates an invite for a server owned by the current user.
pub(crate) async fn create_server_invite(
    server_id: String,
    max_uses: Option<u32>,
    expires_in_days: Option<u32>,
) -> Result<String, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = Request::post(&auth_api::url(&format!("/servers/{server_id}/invites")))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&CreateServerInviteRequest {
            max_uses,
            expires_in_days,
        })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return response
            .json::<CreateServerInviteResponse>()
            .await
            .map(|response| response.code)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}
