//! Счётчики личных сообщений в памяти для локального запуска и тестов.

use std::collections::BTreeMap;
use std::sync::Mutex;

use anyhow::anyhow;
use chrono::{DateTime, Utc};

use crate::features::social::domain::DmMessagesPerMinute;
use crate::features::social::infrastructure::InMemorySocialStore;

/// Считает все личные сообщения, включая мягко удалённые.
pub(super) fn count_dm_messages(
    messages: &Mutex<Vec<crate::features::social::domain::DmMessage>>,
) -> anyhow::Result<u64> {
    Ok(messages.lock().map_err(|_| poisoned())?.len() as u64)
}

/// Возвращает число личных сообщений по минутам в полуинтервале `(since, until]`.
pub(super) fn count_dm_messages_per_minute(
    store: &InMemorySocialStore,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> anyhow::Result<Vec<DmMessagesPerMinute>> {
    let messages = store.messages.lock().map_err(|_| poisoned())?;
    let mut per_minute: BTreeMap<i64, u64> = Default::default();

    for message in messages
        .iter()
        .filter(|message| message.created_at > since && message.created_at <= until)
    {
        *per_minute
            .entry(message.created_at.timestamp() - message.created_at.timestamp() % 60)
            .or_default() += 1;
    }

    Ok(per_minute
        .into_iter()
        .map(|(minute, messages)| DmMessagesPerMinute {
            minute: DateTime::from_timestamp(minute, 0).unwrap_or(since),
            messages,
        })
        .collect())
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory social store lock poisoned")
}
