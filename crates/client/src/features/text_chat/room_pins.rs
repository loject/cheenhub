//! Состояние закреплений комнаты, загрузка и согласование realtime-событий.

use cheenhub_contracts::realtime::{PinnedTextMessage, TextChatMessage};
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};

use super::pins::{self, RoomPinEvent};
use super::pins_panel::PinPreview;

/// Адрес комнаты и транспорт её операций закрепления.
#[derive(Clone)]
pub(super) struct RoomPinsTarget {
    /// Realtime-подключение для запросов.
    pub(super) realtime: RealtimeHandle,
    /// Сервер комнаты.
    pub(super) server_id: String,
    /// Идентификатор комнаты.
    pub(super) room_id: String,
}

/// Сигналы закреплений, принадлежащие одному экземпляру панели комнаты.
#[derive(Clone, Copy)]
pub(super) struct RoomPinsState {
    pins: Signal<Vec<PinnedTextMessage>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    action_error: Signal<Option<String>>,
    limit: Signal<u32>,
    busy_message_id: Signal<Option<String>>,
    revision: Signal<u64>,
    refresh_id: Signal<u64>,
}

/// Подписывается на изменения закреплений и восстанавливает их после переподключения.
pub(super) fn use_room_pins(target: RoomPinsTarget) -> RoomPinsState {
    let state = RoomPinsState {
        pins: use_signal(Vec::new),
        loading: use_signal(|| true),
        error: use_signal(|| None),
        action_error: use_signal(|| None),
        limit: use_signal(|| 50),
        busy_message_id: use_signal(|| None),
        revision: use_signal(|| 0),
        refresh_id: use_signal(|| 0),
    };
    use_hook(move || {
        let mut statuses = target.realtime.subscribe_connection_status();
        let mut events = pins::subscribe_pin_events(&target.realtime);
        let refresh_target = target.clone();
        let event_target = target.clone();
        spawn(async move {
            while let Some(status) = statuses.next().await {
                if matches!(status, RealtimeConnectionStatus::Connected(_)) {
                    state.refresh(refresh_target.clone());
                }
            }
        });
        spawn(async move {
            while let Some(event) = events.next().await {
                match event {
                    RoomPinEvent::Pinned(pin)
                        if pin.message.server_id == event_target.server_id
                            && pin.message.room_id == event_target.room_id =>
                    {
                        state.upsert(pin);
                    }
                    RoomPinEvent::Unpinned(unpin)
                        if unpin.server_id == event_target.server_id
                            && unpin.room_id == event_target.room_id =>
                    {
                        state.remove(&unpin.message_id);
                    }
                    _ => {}
                }
            }
        });
    });
    state
}

impl RoomPinsState {
    /// Текущие идентификаторы закреплений для отображения отметок в истории.
    pub(super) fn ids(self) -> Vec<String> {
        self.pins
            .read()
            .iter()
            .map(|pin| pin.message.id.clone())
            .collect()
    }

    /// Список данных, не привязанных к контрактам серверной комнаты.
    pub(super) fn previews(self) -> Vec<PinPreview> {
        self.pins.read().iter().map(PinPreview::from).collect()
    }

    /// Признак загрузки списка.
    pub(super) fn loading(self) -> bool {
        *self.loading.read()
    }
    /// Ошибка загрузки, при которой можно повторить запрос.
    pub(super) fn error(self) -> Option<String> {
        self.error.read().clone()
    }
    /// Ошибка действия над одним сообщением.
    pub(super) fn action_error(self) -> Option<String> {
        self.action_error.read().clone()
    }
    /// Текущий серверный лимит закреплений.
    pub(super) fn limit(self) -> u32 {
        *self.limit.read()
    }
    /// Сообщение, над которым выполняется запрос.
    pub(super) fn busy(self) -> Option<String> {
        self.busy_message_id.read().clone()
    }

    /// Загружает список и не допускает перезаписи новых realtime-событий старым снимком.
    pub(super) fn refresh(mut self, target: RoomPinsTarget) {
        let next_refresh_id = self.refresh_id.peek().wrapping_add(1);
        self.refresh_id.set(next_refresh_id);
        let refresh_id = *self.refresh_id.peek();
        self.loading.set(true);
        self.error.set(None);
        spawn(async move {
            let mut attempts = 0;
            loop {
                let revision = *self.revision.peek();
                let result = pins::load_pins(
                    &target.realtime,
                    target.server_id.clone(),
                    target.room_id.clone(),
                )
                .await;
                if *self.refresh_id.peek() != refresh_id {
                    return;
                }
                if *self.revision.peek() != revision {
                    attempts += 1;
                    if attempts < 4 {
                        continue;
                    }
                    warn!(server_id = %target.server_id, room_id = %target.room_id,
                        "room pins changed repeatedly while loading snapshot");
                    self.error.set(Some(
                        "Закрепления обновляются слишком часто. Повтори загрузку.".to_owned(),
                    ));
                    break;
                }
                match result {
                    Ok(snapshot) => {
                        debug!(server_id = %target.server_id, room_id = %target.room_id,
                            count = snapshot.messages.len(), "loaded room message pins");
                        self.pins.set(snapshot.messages);
                        self.limit.set(snapshot.limit);
                    }
                    Err(error) => {
                        warn!(server_id = %target.server_id, room_id = %target.room_id,
                            %error, "failed to refresh room message pins");
                        self.error.set(Some(
                            "Не удалось загрузить закрепления. Попробуй ещё раз.".to_owned(),
                        ));
                    }
                }
                break;
            }
            self.loading.set(false);
        });
    }

    /// Закрепляет или открепляет сообщение в зависимости от текущего состояния списка.
    pub(super) fn toggle(self, target: RoomPinsTarget, message_id: String) {
        let is_pinned = self
            .pins
            .peek()
            .iter()
            .any(|pin| pin.message.id == message_id);
        self.change(target, message_id, is_pinned);
    }

    /// Открепляет сообщение из открытого списка закреплений.
    pub(super) fn unpin(self, target: RoomPinsTarget, message_id: String) {
        self.change(target, message_id, true);
    }

    fn change(mut self, target: RoomPinsTarget, message_id: String, unpin: bool) {
        if self.busy_message_id.peek().is_some() {
            return;
        }
        self.action_error.set(None);
        self.busy_message_id.set(Some(message_id.clone()));
        spawn(async move {
            if unpin {
                match pins::unpin_message(
                    &target.realtime,
                    target.server_id.clone(),
                    target.room_id.clone(),
                    message_id.clone(),
                )
                .await
                {
                    Ok(_) => self.remove(&message_id),
                    Err(error) => {
                        warn!(%message_id, %error, "failed to unpin message from room UI");
                        self.action_error.set(Some(
                            "Не удалось открепить сообщение. Попробуй ещё раз.".to_owned(),
                        ));
                        self.refresh(target);
                    }
                }
            } else {
                match pins::pin_message(
                    &target.realtime,
                    target.server_id.clone(),
                    target.room_id.clone(),
                    message_id.clone(),
                )
                .await
                {
                    Ok(result) => self.upsert(result.pinned),
                    Err(error) => {
                        warn!(%message_id, %error, "failed to pin message from room UI");
                        self.action_error.set(Some(
                            "Не удалось закрепить сообщение. Проверь список и повтори попытку."
                                .to_owned(),
                        ));
                        self.refresh(target);
                    }
                }
            }
            self.busy_message_id.set(None);
        });
    }

    fn upsert(mut self, pin: PinnedTextMessage) {
        let mut all = self.pins.read().to_vec();
        all.retain(|entry| entry.message.id != pin.message.id);
        all.push(pin);
        all.sort_by(|left, right| {
            right
                .pinned_at
                .cmp(&left.pinned_at)
                .then_with(|| right.message.id.cmp(&left.message.id))
        });
        self.pins.set(all);
        let next_revision = self.revision.peek().wrapping_add(1);
        self.revision.set(next_revision);
    }

    /// Обновляет текст закреплённого сообщения по событию редакции.
    pub(super) fn apply_edited_message(mut self, message: &TextChatMessage) {
        let mut pins = self.pins.read().to_vec();
        let mut changed = false;
        for pin in &mut pins {
            changed |= super::messages::apply_message_edit(
                std::slice::from_mut(&mut pin.message),
                message.clone(),
            );
        }
        if changed {
            self.pins.set(pins);
            let next_revision = self.revision.peek().wrapping_add(1);
            self.revision.set(next_revision);
        }
    }

    /// Убирает закрепление из локального списка после открепления или удаления сообщения.
    pub(super) fn remove(mut self, message_id: &str) {
        let mut all = self.pins.read().to_vec();
        all.retain(|pin| pin.message.id != message_id);
        self.pins.set(all);
        let next_revision = self.revision.peek().wrapping_add(1);
        self.revision.set(next_revision);
    }
}

impl From<&PinnedTextMessage> for PinPreview {
    fn from(pin: &PinnedTextMessage) -> Self {
        let body = if pin.message.body.trim().is_empty() {
            if pin.message.attachments.is_empty() {
                "Сообщение без текста".to_owned()
            } else {
                "Изображение".to_owned()
            }
        } else {
            pin.message.body.clone()
        };
        let pinned_at = chrono::DateTime::parse_from_rfc3339(&pin.pinned_at)
            .map(|timestamp| timestamp.format("%d.%m.%Y %H:%M").to_string())
            .unwrap_or_else(|_| pin.pinned_at.clone());
        Self {
            message_id: pin.message.id.clone(),
            author: pin.message.author_nickname.clone(),
            body,
            pinned_at,
            pinned_at_iso: pin.pinned_at.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
