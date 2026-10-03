//! Минимальный уровень журналирования хоста.
//!
//! Модуль читает сохранённый уровень из хранилища хоста и передаёт его
//! модулю `telemetry`, который владеет фильтром процесса. Правила доступа
//! и разбора ошибок берутся из `application`, поэтому этот модуль не содержит
//! собственной логики авторизации.

use cheenhub_contracts::rest::{
    HostLogLevel, HostLogSettingsResponse, UpdateHostLogSettingsRequest,
};
use chrono::{SecondsFormat, Utc};

use crate::state::AppState;
use crate::telemetry;

use super::application::{self, HostSettingsError};
use super::domain::{HostLogSettings, LogLevel};

// Фильтр принадлежит процессу, поэтому общий lock охватывает все AppState.
static SETTINGS_CHANGE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Возвращает текущий минимальный уровень журнала только владельцу хоста.
pub(crate) async fn settings(
    state: &AppState,
    access_token: &str,
) -> Result<HostLogSettingsResponse, HostSettingsError> {
    let user_id = application::require_host_owner(state, access_token).await?;
    let settings = state.host_settings_store.load_log_settings().await?;
    tracing::debug!(%user_id, ?settings.min_level, "loaded host log settings");
    Ok(response(settings))
}

/// Сохраняет выбранный уровень, применяет его к процессу и аудирует изменение.
///
/// Изменения сериализуются вместе с сохранением; сбой возвращает предыдущий
/// фактический фильтр. Рабочая задача завершает запись даже при отмене HTTP-запроса,
/// чтобы подтверждённая базой настройка и фильтр не разошлись из-за отмены future.
pub(crate) async fn update_settings(
    state: &AppState,
    access_token: &str,
    request: UpdateHostLogSettingsRequest,
) -> Result<HostLogSettingsResponse, HostSettingsError> {
    let user_id = application::require_host_owner(state, access_token).await?;
    let level = request.min_level.map(to_domain_level);

    let store = state.host_settings_store.clone();

    tokio::spawn(async move {
        let _change = SETTINGS_CHANGE.lock().await;
        let filter_change = telemetry::change_log_level(level.map(LogLevel::as_str)).map_err(|error| {
            tracing::error!(%user_id, ?level, %error, "failed to apply host log level");
            HostSettingsError::Internal(error)
        })?;

        let settings = store.save_log_settings(
            HostLogSettings { min_level: level, updated_at: None },
            user_id,
            Utc::now(),
        ).await.map_err(|error| {
            tracing::error!(%user_id, ?level, %error, "failed to save host log level; restoring previous filter");
            HostSettingsError::Internal(error)
        })?;
        filter_change.commit();
        tracing::info!(%user_id, ?level, "updated host log level");
        Ok(response(settings))
    }).await.map_err(|error| {
        tracing::error!(%user_id, %error, "host log settings task failed");
        HostSettingsError::Internal(error.into())
    })?
}

/// Восстанавливает сохранённый уровень при старте процесса.
///
/// Сбой хранилища или фильтра не прерывает запуск: сервер продолжает работать
/// с фильтром из конфигурации, а причина попадает в журнал.
pub(crate) async fn restore(state: &AppState) {
    let _change = SETTINGS_CHANGE.lock().await;
    let settings = match state.host_settings_store.load_log_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(
                %error,
                "failed to load stored host log level; keeping startup filter"
            );
            return;
        }
    };

    let Some(level) = settings.min_level else {
        return;
    };
    match telemetry::set_log_level(Some(level.as_str())) {
        Ok(()) => tracing::info!(level = level.as_str(), "restored stored host log level"),
        Err(error) => tracing::error!(
            %error,
            level = level.as_str(),
            "failed to apply stored host log level; keeping startup filter"
        ),
    }
}

/// Приводит доменную настройку к контракту REST.
///
/// `None` в ответе означает, что сервер использует фильтр запуска, а не
/// случайно потерянное значение.
fn response(settings: HostLogSettings) -> HostLogSettingsResponse {
    HostLogSettingsResponse {
        min_level: settings.min_level.map(to_contract_level),
        updated_at: settings
            .updated_at
            .map(|updated_at| updated_at.to_rfc3339_opts(SecondsFormat::Secs, true)),
    }
}

fn to_contract_level(level: LogLevel) -> HostLogLevel {
    match level {
        LogLevel::Error => HostLogLevel::Error,
        LogLevel::Warn => HostLogLevel::Warn,
        LogLevel::Info => HostLogLevel::Info,
        LogLevel::Debug => HostLogLevel::Debug,
        LogLevel::Trace => HostLogLevel::Trace,
    }
}

fn to_domain_level(level: HostLogLevel) -> LogLevel {
    match level {
        HostLogLevel::Error => LogLevel::Error,
        HostLogLevel::Warn => LogLevel::Warn,
        HostLogLevel::Info => LogLevel::Info,
        HostLogLevel::Debug => LogLevel::Debug,
        HostLogLevel::Trace => LogLevel::Trace,
    }
}

#[cfg(test)]
mod tests;
