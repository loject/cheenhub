//! Клиентские realtime-запросы закреплений и подписка на изменения.

use cheenhub_contracts::realtime::{
    ListPinnedMessages, PinMessage, PinMessageAccepted, PinnedMessages, PinnedTextMessage,
    RealtimeEnvelope, RealtimeKind, RealtimeModule, TextChatKind, UnpinMessage,
    UnpinMessageAccepted,
};
use dioxus::prelude::{debug, warn};
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Уведомление об изменении закреплений серверной комнаты.
pub(super) enum RoomPinEvent {
    /// Участник закрепил существующее сообщение.
    Pinned(Box<PinnedTextMessage>),
    /// Участник снял закрепление с сообщения.
    Unpinned(UnpinMessageAccepted),
}

/// Загружает актуальный список закреплений доступной комнаты.
pub(super) async fn load_pins(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<PinnedMessages, RealtimeError> {
    let result = realtime
        .request_one_shot(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::ListPinnedMessages),
            ListPinnedMessages {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
            },
        )
        .await;
    if let Err(error) = &result {
        warn!(%server_id, %room_id, %error, "failed to load pinned room messages");
    }
    result
}

/// Запрашивает закрепление существующего сообщения и ожидает подтверждения сервера.
pub(super) async fn pin_message(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    message_id: String,
) -> Result<PinMessageAccepted, RealtimeError> {
    let result = realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::PinMessage),
            PinMessage {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                message_id: message_id.clone(),
            },
        )
        .await;
    match &result {
        Ok(_) => debug!(%server_id, %room_id, %message_id, "room message pinned"),
        Err(error) => {
            warn!(%server_id, %room_id, %message_id, %error, "failed to pin room message")
        }
    }
    result
}

/// Запрашивает снятие закрепления, не затрагивая сообщение в истории.
pub(super) async fn unpin_message(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    message_id: String,
) -> Result<UnpinMessageAccepted, RealtimeError> {
    let result = realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::UnpinMessage),
            UnpinMessage {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                message_id: message_id.clone(),
            },
        )
        .await;
    match &result {
        Ok(_) => debug!(%server_id, %room_id, %message_id, "room message unpinned"),
        Err(error) => {
            warn!(%server_id, %room_id, %message_id, %error, "failed to unpin room message")
        }
    }
    result
}

/// Подписывается на изменения закреплений без вмешательства в поток обычного чата.
pub(super) fn subscribe_pin_events(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<RoomPinEvent> {
    let mut events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        while let Some(envelope) = events.next().await {
            if let Some(event) = decode_pin_event(envelope)
                && sender.unbounded_send(event).is_err()
            {
                break;
            }
        }
    });

    receiver
}

fn decode_pin_event(envelope: RealtimeEnvelope) -> Option<RoomPinEvent> {
    if envelope.module != RealtimeModule::TextChat {
        return None;
    }
    match envelope.kind {
        RealtimeKind::TextChat(TextChatKind::MessagePinned) => {
            let pinned = serde_json::from_value::<PinnedTextMessage>(envelope.payload).ok()?;
            Some(RoomPinEvent::Pinned(Box::new(pinned)))
        }
        RealtimeKind::TextChat(TextChatKind::MessageUnpinned) => {
            let unpinned = serde_json::from_value::<UnpinMessageAccepted>(envelope.payload).ok()?;
            Some(RoomPinEvent::Unpinned(unpinned))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
