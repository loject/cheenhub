//! Сводная статистика CheenHub для владельца хоста.
//!
//! Модуль только собирает счётчики из хранилищ разных фич: сами данные
//! остаются в своих границах, поэтому сбой любого источника возвращается
//! ошибкой запроса, а не частичным ответом.

use cheenhub_contracts::rest::{HostMessagesPerMinuteSample, HostStatsResponse};
use chrono::{DateTime, Duration as ChronoDuration, Utc};

use crate::state::AppState;

use super::application::{self, HostSettingsError};

/// Глубина окна графика сообщений: последние сутки.
const HISTORY_WINDOW_HOURS: i64 = 24;

/// Возвращает счётчики пользователей, серверов, комнат и сообщений.
pub(crate) async fn stats(
    state: &AppState,
    access_token: &str,
) -> Result<HostStatsResponse, HostSettingsError> {
    let user_id = application::require_host_owner(state, access_token).await?;
    let until = Utc::now();
    let since = until - ChronoDuration::hours(HISTORY_WINDOW_HOURS);

    let users_total = state.auth_store.count_registered_users().await?;
    let servers_total = state.server_store.count_servers().await?;
    let rooms_total = state.server_store.count_all_rooms().await?;
    let room_messages_total = state.text_chat_store.count_text_messages().await?;
    let direct_messages_total = state.social_store.count_dm_messages().await?;
    let room_messages = state
        .text_chat_store
        .count_messages_per_minute(since, until)
        .await?;
    let direct_messages = state
        .social_store
        .count_dm_messages_per_minute(since, until)
        .await?;

    tracing::debug!(
        %user_id,
        users_total,
        servers_total,
        rooms_total,
        room_messages_total,
        direct_messages_total,
        busy_room_minutes = room_messages.len(),
        busy_direct_minutes = direct_messages.len(),
        "returned host statistics dashboard snapshot"
    );

    Ok(HostStatsResponse {
        messages_window_end_unix_ms: until.timestamp_millis(),
        users_total,
        servers_total,
        rooms_total,
        room_messages_total,
        direct_messages_total,
        room_messages_per_minute: per_minute_samples(
            room_messages
                .into_iter()
                .map(|sample| (sample.minute, sample.messages)),
        ),
        direct_messages_per_minute: per_minute_samples(
            direct_messages
                .into_iter()
                .map(|sample| (sample.minute, sample.messages)),
        ),
    })
}

/// Приводит поминутные счётчики хранилищ к контракту REST.
///
/// Оба хранилища отдают свои доменные структуры, поэтому в контракт попадает
/// только пара «минута, число сообщений».
fn per_minute_samples(
    samples: impl IntoIterator<Item = (DateTime<Utc>, u64)>,
) -> Vec<HostMessagesPerMinuteSample> {
    samples
        .into_iter()
        .map(|(minute, messages)| HostMessagesPerMinuteSample {
            minute_unix_ms: minute.timestamp_millis(),
            messages,
        })
        .collect()
}

#[cfg(test)]
mod tests;
