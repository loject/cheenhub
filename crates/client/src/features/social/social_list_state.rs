//! Состояние поиска и курсорных страниц объединённого списка друзей и ЛС.

use std::collections::HashMap;
use std::time::Duration;

use cheenhub_contracts::presence::UserPresence;
use cheenhub_contracts::realtime::{PresenceChanged, SocialListEntrySummary};
use cheenhub_contracts::rest::UserRelationStatus;
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_duration;

use super::realtime;

const SEARCH_DEBOUNCE_MS: u64 = 220;

/// Локальная выборка, состояние запроса и защита от ответов устаревших страниц.
#[derive(Clone, Copy)]
pub(super) struct SocialListPageState {
    pub(super) entries: Signal<Option<Vec<SocialListEntrySummary>>>,
    /// События присутствия, полученные после начала последней полной загрузки.
    pub(super) presence_overrides: Signal<HashMap<String, UserPresence>>,
    pub(super) query: Signal<String>,
    pub(super) next_cursor: Signal<Option<String>>,
    pub(super) has_more: Signal<bool>,
    pub(super) is_loading: Signal<bool>,
    pub(super) is_loading_more: Signal<bool>,
    pub(super) error: Signal<Option<String>>,
    pub(super) generation: Signal<u64>,
}

/// Перезапускает поиск с первой страницы; старые ответы не применяются.
///
/// Для ввода в поле поиска используется короткая задержка, при обновлении
/// по social-событию запрос отправляется немедленно.
pub(super) fn load_social_list(
    realtime: RealtimeHandle,
    mut state: SocialListPageState,
    debounce: bool,
) {
    let generation = (state.generation)().wrapping_add(1);
    state.generation.set(generation);
    // Старые события уже учтены свежим серверным снимком; сохраняем только новые.
    state.presence_overrides.write().clear();
    state.next_cursor.set(None);
    state.has_more.set(false);
    state.is_loading_more.set(false);
    state.is_loading.set(true);
    state.error.set(None);
    if debounce {
        state.entries.set(None);
    }
    let query = request_query(&(state.query)());
    spawn(async move {
        if debounce {
            sleep_duration(Duration::from_millis(SEARCH_DEBOUNCE_MS)).await;
        }
        if (state.generation)() != generation {
            return;
        }
        let result = realtime::list_social_entries(&realtime, query, None).await;
        if (state.generation)() != generation {
            debug!(generation, "ignored outdated unified social list response");
            return;
        }
        match result {
            Ok(page) => {
                debug!(
                    generation,
                    count = page.entries.len(),
                    has_more = page.has_more,
                    "loaded first unified social page"
                );
                let mut entries = page.entries;
                apply_presence_overrides(&mut entries, &state.presence_overrides.read());
                state.entries.set(Some(entries));
                state.next_cursor.set(page.next_cursor);
                state.has_more.set(page.has_more);
            }
            Err(error) => {
                warn!(%error, "failed to load unified social list");
                state.error.set(Some(error));
            }
        }
        state.is_loading.set(false);
    });
}

/// Продолжает тот же поиск, сохраняя порядок и уникальность пользователей.
pub(super) fn load_more_social_list(realtime: RealtimeHandle, mut state: SocialListPageState) {
    if (state.is_loading)() || (state.is_loading_more)() || !(state.has_more)() {
        return;
    }
    let Some(cursor) = (state.next_cursor)() else {
        state.has_more.set(false);
        return;
    };
    let generation = (state.generation)();
    let query = request_query(&(state.query)());
    state.is_loading_more.set(true);
    state.error.set(None);
    spawn(async move {
        let result = realtime::list_social_entries(&realtime, query, Some(cursor)).await;
        if (state.generation)() != generation {
            debug!(generation, "ignored outdated unified social continuation");
            return;
        }
        match result {
            Ok(page) => {
                let mut loaded =
                    merge_social_entries((state.entries)().unwrap_or_default(), page.entries);
                apply_presence_overrides(&mut loaded, &state.presence_overrides.read());
                debug!(
                    generation,
                    count = loaded.len(),
                    has_more = page.has_more,
                    "loaded another unified social page"
                );
                state.entries.set(Some(loaded));
                state.next_cursor.set(page.next_cursor);
                state.has_more.set(page.has_more);
            }
            Err(error) => {
                warn!(%error, "failed to load another unified social page");
                state.error.set(Some(error));
            }
        }
        state.is_loading_more.set(false);
    });
}

/// Запоминает событие на случай, если соответствующая страница ещё загружается.
///
/// Одновременно обновляет уже видимую запись, не раскрывая присутствие не-друзей.
pub(super) fn record_social_presence(
    mut state: SocialListPageState,
    event: &PresenceChanged,
) -> bool {
    state
        .presence_overrides
        .write()
        .insert(event.user_id.clone(), event.presence.clone());
    let mut current_entries = state.entries.write();
    current_entries
        .as_mut()
        .is_some_and(|entries| apply_social_presence(entries, event))
}

/// Более свежие realtime-события побеждают снимок страницы, полученный позднее.
pub(super) fn apply_presence_overrides(
    entries: &mut [SocialListEntrySummary],
    overrides: &HashMap<String, UserPresence>,
) {
    for entry in entries {
        if entry.relation == Some(UserRelationStatus::Friends)
            && let Some(presence) = overrides.get(&entry.user_id)
        {
            entry.presence = Some(presence.clone());
        }
    }
}

/// Применяет событие присутствия только к другу из уже загруженных страниц.
pub(super) fn apply_social_presence(
    entries: &mut [SocialListEntrySummary],
    event: &PresenceChanged,
) -> bool {
    let Some(entry) = entries.iter_mut().find(|entry| {
        entry.user_id == event.user_id && entry.relation == Some(UserRelationStatus::Friends)
    }) else {
        return false;
    };
    entry.presence = Some(event.presence.clone());
    true
}

/// Склеивает последовательные страницы без повторения одного собеседника.
pub(super) fn merge_social_entries(
    mut existing: Vec<SocialListEntrySummary>,
    next: Vec<SocialListEntrySummary>,
) -> Vec<SocialListEntrySummary> {
    for entry in next {
        if !existing.iter().any(|saved| saved.user_id == entry.user_id) {
            existing.push(entry);
        }
    }
    existing
}

fn request_query(query: &str) -> Option<String> {
    let normalized = query.trim();
    (!normalized.is_empty()).then(|| normalized.to_owned())
}
