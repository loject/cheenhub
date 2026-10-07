//! Настройки доступности регистрации пользователей.
//!
//! Модуль отдаёт владельцам хоста управление настройками и предоставляет
//! приложению аутентификации актуальные серверные проверки регистрации.

use cheenhub_contracts::rest::{
    HostRegistrationSettingsResponse, UpdateHostRegistrationSettingsRequest,
};
use chrono::Utc;

use crate::state::AppState;

use super::application::{self, HostSettingsError};
use super::domain::HostRegistrationSettings;

/// Возвращает настройки регистрации только владельцу хоста.
pub(crate) async fn settings(
    state: &AppState,
    access_token: &str,
) -> Result<HostRegistrationSettingsResponse, HostSettingsError> {
    let user_id = application::require_host_owner(state, access_token).await?;
    let settings = state
        .host_settings_store
        .load_registration_settings()
        .await?;
    tracing::debug!(%user_id, ?settings, "loaded host registration settings");
    Ok(response(settings))
}

/// Сохраняет оба переключателя регистрации только для владельца хоста.
pub(crate) async fn update_settings(
    state: &AppState,
    access_token: &str,
    request: UpdateHostRegistrationSettingsRequest,
) -> Result<HostRegistrationSettingsResponse, HostSettingsError> {
    let user_id = application::require_host_owner(state, access_token).await?;
    let settings = HostRegistrationSettings {
        registration_enabled: request.registration_enabled,
        email_password_registration_enabled: request.email_password_registration_enabled,
    };
    let settings = state
        .host_settings_store
        .save_registration_settings(settings, user_id, Utc::now())
        .await?;
    tracing::info!(
        %user_id,
        registration_enabled = settings.registration_enabled,
        email_password_registration_enabled = settings.email_password_registration_enabled,
        "updated host registration settings"
    );
    Ok(response(settings))
}

/// Проверяет, разрешена ли регистрация по email и паролю.
pub(crate) async fn email_password_registration_enabled(state: &AppState) -> anyhow::Result<bool> {
    let settings = state
        .host_settings_store
        .load_registration_settings()
        .await?;
    Ok(settings.registration_enabled && settings.email_password_registration_enabled)
}

/// Проверяет, разрешено ли создание аккаунта через внешний вход.
pub(crate) async fn oauth_registration_enabled(state: &AppState) -> anyhow::Result<bool> {
    Ok(state
        .host_settings_store
        .load_registration_settings()
        .await?
        .registration_enabled)
}

fn response(settings: HostRegistrationSettings) -> HostRegistrationSettingsResponse {
    HostRegistrationSettingsResponse {
        registration_enabled: settings.registration_enabled,
        email_password_registration_enabled: settings.email_password_registration_enabled,
    }
}
