//! Text chat realtime helpers.

use cheenhub_contracts::realtime::{
    DeleteMessage, DeleteMessageAccepted, LoadRoomHistory, MessageDeletedPayload, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, RoomHistory, SendMessage, SendMessageAccepted, TextChatKind,
    TextChatMessage,
};
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Inbound text chat event delivered via WebSocket subscription.
pub(crate) enum TextChatEvent {
    /// A new message was created.
    MessageCreated(TextChatMessage),
    /// A message was removed by its author.
    MessageDeleted(MessageDeletedPayload),
}

/// Loads latest text chat history for a room.
pub(crate) async fn load_room_history(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    before_message_id: Option<String>,
) -> Result<RoomHistory, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::LoadRoomHistory),
            LoadRoomHistory {
                server_id,
                room_id,
                before_message_id,
            },
        )
        .await
}

/// Sends one text chat message.
pub(crate) async fn send_text_message(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    body: String,
) -> Result<SendMessageAccepted, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::SendMessage),
            SendMessage {
                server_id,
                room_id,
                body,
            },
        )
        .await
}

/// Soft-deletes one of the user's own messages.
pub(crate) async fn delete_text_message(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    message_id: String,
) -> Result<DeleteMessageAccepted, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::DeleteMessage),
            DeleteMessage {
                server_id,
                room_id,
                message_id,
            },
        )
        .await
}

/// Subscribes to inbound text chat events (creates and deletes) for this tab.
pub(crate) fn subscribe_text_chat(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<TextChatEvent> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(event) = decode_text_chat_event(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    });

    receiver
}

fn decode_text_chat_event(envelope: RealtimeEnvelope) -> Option<TextChatEvent> {
    if envelope.module != RealtimeModule::TextChat {
        return None;
    }
    match envelope.kind {
        RealtimeKind::TextChat(TextChatKind::MessageCreated) => {
            let message = serde_json::from_value::<TextChatMessage>(envelope.payload).ok()?;
            Some(TextChatEvent::MessageCreated(message))
        }
        RealtimeKind::TextChat(TextChatKind::MessageDeleted) => {
            let payload =
                serde_json::from_value::<MessageDeletedPayload>(envelope.payload).ok()?;
            Some(TextChatEvent::MessageDeleted(payload))
        }
        _ => None,
    }
}
