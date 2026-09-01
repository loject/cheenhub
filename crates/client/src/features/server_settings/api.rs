//! Server settings REST client helpers.

use cheenhub_contracts::rest::{
    ServerSummary, ServerVoiceSettings, UpdateServerAvatarResponse, UpdateServerRequest,
    UpdateServerResponse,
};

use crate::features::auth::api as auth_api;

/// Updates a server profile.
pub(super) async fn update_server(
    server_id: String,
    request: UpdateServerRequest,
) -> Result<ServerSummary, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = auth_api::put(&format!("/servers/{server_id}"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&request)
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.status().is_success() {
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
    let response = auth_api::put(&format!("/servers/{server_id}/avatar"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.status().is_success() {
        return response
            .json::<UpdateServerAvatarResponse>()
            .await
            .map(|response| response.server)
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Загружает настройки качества голоса сервера.
pub(super) async fn get_voice_settings(server_id: String) -> Result<ServerVoiceSettings, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = auth_api::get(&format!("/servers/{server_id}/voice-settings"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.status().is_success() {
        return response
            .json::<ServerVoiceSettings>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}

/// Обновляет настройки качества голоса сервера.
pub(super) async fn update_voice_settings(
    server_id: String,
    request: ServerVoiceSettings,
) -> Result<ServerVoiceSettings, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = auth_api::put(&format!("/servers/{server_id}/voice-settings"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(&request)
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.status().is_success() {
        return response
            .json::<ServerVoiceSettings>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(auth_api::read_error(response).await)
}
