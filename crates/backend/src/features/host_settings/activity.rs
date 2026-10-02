//! Сценарии мониторинга активности голосового чата для владельца хоста.

use chrono::{DateTime, Duration as ChronoDuration, Utc};

use cheenhub_contracts::rest::{
    HostVoiceActivityHistoryResponse, HostVoiceActivityResponse, HostVoiceActivitySample,
};

use crate::features::voice_chat::application as voice_chat;
use crate::state::AppState;

use super::application::{self, HostSettingsError};

/// Возвращает текущие голосовые подключения и видеоисточники только владельцу хоста.
///
/// Снимок строится из памяти процесса и не зависит от базы данных, поэтому
/// недоступность истории не скрывает текущую активность.
pub(crate) async fn activity(
    state: &AppState,
    access_token: &str,
) -> Result<HostVoiceActivityResponse, HostSettingsError> {
    let user_id = application::require_host_owner(state, access_token).await?;
    let snapshot = voice_chat::activity_snapshot(state).await;
    tracing::debug!(
        %user_id,
        voice_connections = snapshot.voice_connections,
        video_sources = snapshot.video_sources,
        "returned host voice activity snapshot"
    );
    Ok(HostVoiceActivityResponse {
        voice_connections: snapshot.voice_connections,
        video_sources: snapshot.video_sources,
    })
}

/// Возвращает историю активности за последние 24 часа.
///
/// Недоступность базы данных отражается флагом `available`, а не ошибкой
/// запроса: текущая активность остаётся доступна при сбое истории.
pub(crate) async fn activity_history(
    state: &AppState,
    access_token: &str,
    after_unix_ms: Option<i64>,
) -> Result<HostVoiceActivityHistoryResponse, HostSettingsError> {
    let user_id = application::require_host_owner(state, access_token).await?;
    let now = Utc::now();
    let since = match after_unix_ms {
        Some(millis) => DateTime::from_timestamp_millis(millis)
            .unwrap_or(now - ChronoDuration::hours(24))
            .max(now - ChronoDuration::hours(24)),
        None => now - ChronoDuration::hours(24),
    };
    let samples = match state
        .host_settings_store
        .load_voice_activity_samples(since, now)
        .await
    {
        Ok(samples) => samples,
        Err(error) => {
            tracing::warn!(
                %error,
                %user_id,
                "host voice activity history is unavailable"
            );
            return Ok(HostVoiceActivityHistoryResponse {
                available: false,
                samples: Vec::new(),
            });
        }
    };
    tracing::debug!(
        %user_id,
        sample_count = samples.len(),
        "returned host voice activity history"
    );
    Ok(HostVoiceActivityHistoryResponse {
        available: true,
        samples: samples
            .into_iter()
            .map(|sample| HostVoiceActivitySample {
                sampled_at_unix_ms: sample.sampled_at.timestamp_millis(),
                voice_connections: sample.voice_connections,
                video_sources: sample.video_sources,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests;
