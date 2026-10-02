//! Согласование загруженной истории после восстановления realtime.

use std::cmp::Ordering;
use std::collections::HashSet;

use cheenhub_contracts::realtime::{RoomHistory, TextChatMessage};
use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::future::{Either, select};
use futures_util::{FutureExt, StreamExt};

use super::history::{HistoryState, HistoryTarget, INITIAL_HISTORY_TIMEOUT};
use super::{realtime, scroll::ScrollCommand, scroll_anchor_runtime::capture_anchor};
use crate::features::realtime::RealtimeError;
use crate::features::runtime::sleep_duration;

/// Сверяет серверные страницы до начала уже загруженной истории.
pub(super) struct HistoryRefresh {
    saved: Vec<TextChatMessage>,
    messages: Vec<TextChatMessage>,
    has_more: bool,
    deleted_ids: HashSet<String>,
}

impl HistoryRefresh {
    /// Запоминает загруженный диапазон до начала синхронизации.
    pub(super) fn new(saved: Vec<TextChatMessage>) -> Self {
        Self {
            saved,
            messages: Vec::new(),
            has_more: false,
            deleted_ids: HashSet::new(),
        }
    }

    /// Добавляет страницу и возвращает cursor, пока загруженный диапазон не восстановлен.
    pub(super) fn push_page(&mut self, history: RoomHistory) -> Option<String> {
        let cursor = history.messages.first().and_then(|oldest| {
            (history.has_more
                && self
                    .saved
                    .first()
                    .is_some_and(|saved| compare_message(oldest, saved).is_gt()))
            .then(|| oldest.id.clone())
        });
        self.has_more = history.has_more;
        let mut messages = history.messages;
        messages.append(&mut self.messages);
        self.messages = messages;
        cursor
    }

    /// Запоминает удаление, пришедшее во время восстановления истории.
    pub(super) fn record_deletion(&mut self, id: String) {
        self.deleted_ids.insert(id);
    }

    /// Применяет удаления сервера и сохраняет сообщения, пришедшие во время загрузки.
    pub(super) fn finish(mut self, current: Vec<TextChatMessage>) -> (Vec<TextChatMessage>, bool) {
        if let Some(oldest) = self.saved.first() {
            let before_trim = self.messages.len();
            self.messages
                .retain(|message| !compare_message(message, oldest).is_lt());
            self.has_more |= self.messages.len() < before_trim;
        }
        let current_ids = current
            .iter()
            .map(|message| message.id.as_str())
            .collect::<HashSet<_>>();
        let saved_ids = self
            .saved
            .iter()
            .map(|message| message.id.as_str())
            .collect::<HashSet<_>>();
        self.messages.retain(|message| {
            !saved_ids.contains(message.id.as_str()) || current_ids.contains(message.id.as_str())
        });
        let mut merged_ids = self
            .messages
            .iter()
            .map(|message| message.id.clone())
            .collect::<HashSet<_>>();
        for message in current {
            if !saved_ids.contains(message.id.as_str()) && merged_ids.insert(message.id.clone()) {
                self.messages.push(message);
            }
        }
        self.messages
            .retain(|message| !self.deleted_ids.contains(&message.id));
        self.messages.sort_by(compare_message);
        self.messages.dedup_by(|left, right| left.id == right.id);
        (self.messages, self.has_more)
    }
}

fn compare_message(left: &TextChatMessage, right: &TextChatMessage) -> Ordering {
    match (
        chrono::DateTime::parse_from_rfc3339(&left.created_at),
        chrono::DateTime::parse_from_rfc3339(&right.created_at),
    ) {
        (Ok(left_time), Ok(right_time)) => left_time.cmp(&right_time),
        _ => left.created_at.cmp(&right.created_at),
    }
    .then_with(|| left.id.cmp(&right.id))
}

/// Восстанавливает историю без потери загруженных страниц и принудительного перехода вниз.
pub(super) async fn refresh_history(target: HistoryTarget, mut state: HistoryState) {
    state.initial_loading.set(true);
    state.history_error.set(None);
    let saved = (state.messages)();
    let mut events = realtime::subscribe_text_chat(&target.realtime);
    let result = {
        let request = load_refresh(&target, saved, &mut events).boxed_local();
        let timeout = sleep_duration(INITIAL_HISTORY_TIMEOUT).boxed_local();
        match select(request, timeout).await {
            Either::Left((result, _)) => Some(result),
            Either::Right(_) => None,
        }
    };
    match result {
        Some(Ok(mut refresh)) => {
            let scroll = match state.list_element.cloned() {
                Some(element) => {
                    capture_anchor(element, state.anchor_elements, &(state.messages)()).await
                }
                None => None,
            };
            while let Some(Some(event)) = events.next().now_or_never() {
                if let realtime::TextChatEvent::MessageDeleted(payload) = event
                    && payload.room_id == target.room_id
                    && payload.server_id == target.server_id
                {
                    refresh.record_deletion(payload.message_id);
                }
            }
            let (messages, has_more) = refresh.finish((state.messages)());
            info!(server_id = %target.server_id, room_id = %target.room_id, messages = messages.len(),
                "restored text chat history after realtime reconnect");
            let ids = messages
                .iter()
                .map(|message| message.id.clone())
                .collect::<Vec<_>>();
            let command = if (state.is_near_bottom)() {
                Some(ScrollCommand::Bottom)
            } else {
                scroll
                    .filter(|anchor| state.anchor_elements.is_current(anchor))
                    .and_then(|mut anchor| {
                        let (message_id, inside_y) = anchor.resolve(&ids)?;
                        anchor.message_id = message_id;
                        anchor.inside_y = inside_y;
                        Some(ScrollCommand::Restore { anchor })
                    })
            };
            state.messages.set(messages);
            state.has_more.set(has_more);
            state.pending_scroll.set(command);
        }
        Some(Err(error)) => {
            warn!(%error, server_id = %target.server_id, room_id = %target.room_id, "failed to restore text chat history");
            state.history_error.set(Some(
                "Не удалось обновить сообщения. Попробуй ещё раз.".to_owned(),
            ));
        }
        None => {
            warn!(server_id = %target.server_id, room_id = %target.room_id, "text chat history restore timed out");
            state.history_error.set(Some(
                "Обновление сообщений заняло слишком много времени. Попробуй ещё раз.".to_owned(),
            ));
        }
    }
    state.initial_loading.set(false);
}

async fn load_refresh(
    target: &HistoryTarget,
    saved: Vec<TextChatMessage>,
    events: &mut mpsc::UnboundedReceiver<realtime::TextChatEvent>,
) -> Result<HistoryRefresh, RealtimeError> {
    let mut refresh = HistoryRefresh::new(saved);
    let mut cursor = None;
    let mut cursors = HashSet::new();
    loop {
        let mut request = realtime::load_room_history(
            &target.realtime,
            target.server_id.clone(),
            target.room_id.clone(),
            cursor,
        )
        .boxed_local();
        let history = loop {
            match select(events.next().boxed_local(), request).await {
                Either::Left((Some(event), pending_request)) => {
                    if let realtime::TextChatEvent::MessageDeleted(payload) = event
                        && payload.room_id == target.room_id
                        && payload.server_id == target.server_id
                    {
                        refresh.record_deletion(payload.message_id);
                    }
                    request = pending_request;
                }
                Either::Left((None, pending_request)) => break pending_request.await?,
                Either::Right((result, _)) => break result?,
            }
        };
        let Some(next_cursor) = refresh.push_page(history) else {
            return Ok(refresh);
        };
        if !cursors.insert(next_cursor.clone()) {
            return Err(RealtimeError::new(
                "Text chat history cursor did not advance",
            ));
        }
        cursor = Some(next_cursor);
    }
}

#[cfg(test)]
#[path = "history_refresh_tests.rs"]
mod tests;
