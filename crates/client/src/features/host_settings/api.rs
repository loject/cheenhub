//! REST-клиент глобальных настроек хоста.

use cheenhub_contracts::rest::{
    GmailConnectionStartResponse, HostAccessResponse, HostEmailSettingsResponse,
    UpdateHostEmailSettingsRequest,
};
use dioxus::prelude::{debug, info, warn};
use reqwest::{Response, StatusCode};
use serde::Serialize;

use crate::features::auth::api as auth_api;

/// Ошибка запроса настроек хоста.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HostSettingsApiError {
    /// Текущий пользователь не является владельцем хоста.
    Forbidden(String),
    /// Остальные сетевые и серверные ошибки.
    Other(String),
}

impl HostSettingsApiError {
    /// Возвращает безопасный для интерфейса текст ошибки.
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Forbidden(message) | Self::Other(message) => message,
        }
    }
}

/// Проверяет доступ текущего пользователя к настройкам хоста.
pub(crate) async fn load_access() -> Result<HostAccessResponse, String> {
    debug!("loading current user host settings access");
    let response = match authorized_get("/host-settings/access").await {
        Ok(response) => response,
        Err(error) => {
            warn!(%error, "failed to request current user host settings access");
            return Err(error);
        }
    };
    if response.status().is_success() {
        let access: HostAccessResponse = response
            .json()
            .await
            .map_err(|_| "Не удалось прочитать права доступа.".to_owned())?;
        info!(
            is_host_owner = access.is_host_owner,
            "current user host settings access loaded"
        );
        return Ok(access);
    }

    let error = auth_api::read_error(response).await;
    warn!(%error, "failed to load current user host settings access");
    Err(error)
}

/// Загружает настройки исходящей почты.
pub(crate) async fn load_email_settings() -> Result<HostEmailSettingsResponse, HostSettingsApiError>
{
    let response = authorized_get("/host-settings/email")
        .await
        .map_err(HostSettingsApiError::Other)?;
    decode_settings(response).await
}

/// Сохраняет настройки исходящей почты.
pub(crate) async fn update_email_settings(
    request: UpdateHostEmailSettingsRequest,
) -> Result<HostEmailSettingsResponse, HostSettingsApiError> {
    let response = authorized_patch("/host-settings/email", &request)
        .await
        .map_err(HostSettingsApiError::Other)?;
    decode_settings(response).await
}

/// Начинает OAuth-подключение Gmail.
pub(crate) async fn start_gmail_connection()
-> Result<GmailConnectionStartResponse, HostSettingsApiError> {
    let response = authorized_post("/host-settings/email/gmail/connect")
        .await
        .map_err(HostSettingsApiError::Other)?;
    if response.status().is_success() {
        return response.json().await.map_err(|_| {
            HostSettingsApiError::Other("Не удалось прочитать URL авторизации Gmail.".to_owned())
        });
    }

    Err(classify_error(response).await)
}

/// Отключает Gmail и удаляет сохранённый refresh token.
pub(crate) async fn disconnect_gmail() -> Result<HostEmailSettingsResponse, HostSettingsApiError> {
    let response = authorized_post("/host-settings/email/gmail/disconnect")
        .await
        .map_err(HostSettingsApiError::Other)?;
    decode_settings(response).await
}

async fn authorized_get(path: &str) -> Result<Response, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = send_get(path, &access_token).await?;
    if response.status() != StatusCode::UNAUTHORIZED {
        return Ok(response);
    }

    let access_token = auth_api::refresh_access_token().await?;
    send_get(path, &access_token).await
}

async fn authorized_patch<T: Serialize + ?Sized>(path: &str, body: &T) -> Result<Response, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = send_patch(path, &access_token, body).await?;
    if response.status() != StatusCode::UNAUTHORIZED {
        return Ok(response);
    }

    let access_token = auth_api::refresh_access_token().await?;
    send_patch(path, &access_token, body).await
}

async fn authorized_post(path: &str) -> Result<Response, String> {
    let access_token = auth_api::fresh_access_token().await?;
    let response = send_post(path, &access_token).await?;
    if response.status() != StatusCode::UNAUTHORIZED {
        return Ok(response);
    }

    let access_token = auth_api::refresh_access_token().await?;
    send_post(path, &access_token).await
}

async fn send_get(path: &str, access_token: &str) -> Result<Response, String> {
    auth_api::get(path)
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn send_patch<T: Serialize + ?Sized>(
    path: &str,
    access_token: &str,
    body: &T,
) -> Result<Response, String> {
    auth_api::patch(path)
        .header("Authorization", &format!("Bearer {access_token}"))
        .json(body)
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn send_post(path: &str, access_token: &str) -> Result<Response, String> {
    auth_api::post(path)
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())
}

async fn decode_settings(
    response: Response,
) -> Result<HostEmailSettingsResponse, HostSettingsApiError> {
    if response.status().is_success() {
        return response.json().await.map_err(|_| {
            HostSettingsApiError::Other("Не удалось прочитать настройки почты.".to_owned())
        });
    }

    Err(classify_error(response).await)
}

async fn classify_error(response: Response) -> HostSettingsApiError {
    let forbidden = response.status() == StatusCode::FORBIDDEN;
    let message = auth_api::read_error(response).await;
    if forbidden {
        HostSettingsApiError::Forbidden(message)
    } else {
        HostSettingsApiError::Other(message)
    }
}
