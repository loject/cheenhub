//! Переход к закреплённому сообщению через существующую пагинацию истории.

use std::collections::HashSet;
use std::time::Duration;

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::runtime::sleep_duration;

use super::history::{HistoryState, HistoryTarget};
use super::realtime;
use super::scroll_anchor_runtime::AnchorElements;

/// Локальное состояние одного перехода к закреплённому сообщению.
#[derive(Clone, Copy)]
pub(super) struct PinNavigation {
    busy: Signal<Option<String>>,
    highlighted: Signal<Option<String>>,
    error: Signal<Option<String>>,
    highlight_revision: Signal<u64>,
}

/// Создаёт независимое состояние навигации для конкретной комнаты.
pub(super) fn use_pin_navigation() -> PinNavigation {
    PinNavigation {
        busy: use_signal(|| None),
        highlighted: use_signal(|| None),
        error: use_signal(|| None),
        highlight_revision: use_signal(|| 0),
    }
}

impl PinNavigation {
    /// Идентификатор сообщения, переход к которому сейчас выполняется.
    pub(super) fn busy(self) -> Option<String> {
        self.busy.read().clone()
    }

    /// Последняя ошибка перехода, видимая в списке закреплений.
    pub(super) fn error(self) -> Option<String> {
        self.error.read().clone()
    }

    /// Идентификатор сообщения, временно подсвеченного в истории.
    pub(super) fn highlighted(self) -> Option<String> {
        self.highlighted.read().clone()
    }

    /// Находит сообщение в загруженном диапазоне или догружает историю до него.
    pub(super) fn jump(
        mut self,
        target: HistoryTarget,
        mut history: HistoryState,
        anchor_elements: AnchorElements,
        message_id: String,
    ) {
        if self.busy.peek().is_some() {
            return;
        }
        if (history.initial_loading)() || (history.older_loading)() {
            self.error.set(Some(
                "История загружается. Попробуй ещё раз чуть позже.".to_owned(),
            ));
            return;
        }
        if history.messages.read().is_empty() {
            self.error.set(Some(
                "История пока недоступна. Повтори попытку после загрузки.".to_owned(),
            ));
            return;
        }
        self.error.set(None);
        self.busy.set(Some(message_id.clone()));
        spawn(async move {
            let already_loaded = history.messages.read().iter().any(|m| m.id == message_id);
            if already_loaded {
                self.show_message(&message_id, history.pending_scroll, anchor_elements);
                self.busy.set(None);
                return;
            }
            if !(history.has_more)() {
                self.error
                    .set(Some("Сообщение не найдено в доступной истории.".to_owned()));
                self.busy.set(None);
                return;
            }
            history.older_loading.set(true);
            let result = seek_older_pages(&target, &history, &message_id).await;
            match result {
                Ok((pages, has_more)) => {
                    // Сохраняем сообщения, полученные обычными событиями во время загрузки.
                    let current = (history.messages)();
                    let merged = merge_pages(pages, current);
                    if merged.iter().any(|message| message.id == message_id) {
                        history.messages.set(merged);
                        history.has_more.set(has_more);
                        self.show_message(&message_id, history.pending_scroll, anchor_elements);
                    } else {
                        self.error
                            .set(Some("Сообщение больше недоступно.".to_owned()));
                    }
                }
                Err(error) => {
                    warn!(server_id = %target.server_id, room_id = %target.room_id,
                        %message_id, %error, "failed to navigate to pinned message");
                    self.error.set(Some(error));
                }
            }
            history.older_loading.set(false);
            self.busy.set(None);
        });
    }

    fn show_message(
        mut self,
        message_id: &str,
        mut pending_scroll: Signal<Option<super::scroll::ScrollCommand>>,
        anchor_elements: AnchorElements,
    ) {
        pending_scroll.set(Some(anchor_elements.jump_command(message_id.to_owned())));
        self.highlighted.set(Some(message_id.to_owned()));
        let next_revision = self.highlight_revision.peek().wrapping_add(1);
        self.highlight_revision.set(next_revision);
        debug!(%message_id, "navigating to pinned message in text history");
        spawn(async move {
            sleep_duration(Duration::from_secs(3)).await;
            if *self.highlight_revision.peek() == next_revision {
                self.highlighted.set(None);
            }
        });
    }
}

async fn seek_older_pages(
    target: &HistoryTarget,
    history: &HistoryState,
    message_id: &str,
) -> Result<(Vec<Vec<TextChatMessage>>, bool), String> {
    let mut cursor = history
        .messages
        .read()
        .first()
        .map(|m| m.id.clone())
        .ok_or_else(|| "История пока недоступна.".to_owned())?;
    let mut visited = HashSet::new();
    let mut pages = Vec::new();
    loop {
        if !visited.insert(cursor.clone()) {
            return Err("Не удалось продолжить загрузку истории. Попробуй ещё раз.".to_owned());
        }
        let page = realtime::load_room_history(
            &target.realtime,
            target.server_id.clone(),
            target.room_id.clone(),
            Some(cursor.clone()),
        )
        .await
        .map_err(|error| {
            warn!(server_id = %target.server_id, room_id = %target.room_id,
                %cursor, %error, "failed to load pinned message history page");
            "Не удалось загрузить старые сообщения. Проверь соединение и повтори.".to_owned()
        })?;
        let next = next_cursor(message_id, &cursor, &page.messages, page.has_more)?;
        let has_more = page.has_more;
        pages.push(page.messages);
        match next {
            None => {
                debug!(server_id = %target.server_id, room_id = %target.room_id,
                    %message_id, pages = pages.len(), "loaded history up to pinned message");
                return Ok((pages, has_more));
            }
            Some(next_cursor) => cursor = next_cursor,
        }
    }
}

/// Определяет, нужно ли загружать ещё одну страницу истории.
fn next_cursor(
    target_id: &str,
    previous_cursor: &str,
    page: &[TextChatMessage],
    has_more: bool,
) -> Result<Option<String>, String> {
    if page.iter().any(|message| message.id == target_id) {
        return Ok(None);
    }
    if !has_more {
        return Err("Сообщение не найдено в доступной истории.".to_owned());
    }
    let Some(oldest) = page.first() else {
        return Err("История не содержит следующей страницы.".to_owned());
    };
    if oldest.id == previous_cursor {
        return Err("История не продвинулась при загрузке.".to_owned());
    }
    Ok(Some(oldest.id.clone()))
}

/// Объединяет страницы от старых к новым, сохраняя поступившие realtime-сообщения.
fn merge_pages(
    pages: Vec<Vec<TextChatMessage>>,
    current: Vec<TextChatMessage>,
) -> Vec<TextChatMessage> {
    let mut seen = current
        .iter()
        .map(|message| message.id.clone())
        .collect::<HashSet<_>>();
    let mut merged = Vec::new();
    for page in pages.into_iter().rev() {
        for message in page {
            if seen.insert(message.id.clone()) {
                merged.push(message);
            }
        }
    }
    merged.extend(current);
    merged
}

#[cfg(test)]
mod tests;
