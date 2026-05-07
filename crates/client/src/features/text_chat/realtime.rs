//! Text chat realtime helpers.

use cheenhub_contracts::realtime::{
    LoadRoomHistory, RealtimeEnvelope, RealtimeKind, RealtimeModule, RoomHistory, SendMessage,
    SendMessageAccepted, TextChatKind, TextChatMessage,
};
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Loads latest text chat history for a room.
pub(crate) async fn load_room_history(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<RoomHistory, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::LoadRoomHistory),
            LoadRoomHistory { server_id, room_id },
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

/// Subscribes to inbound text chat message events for this tab.
pub(crate) fn subscribe_text_chat(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<TextChatMessage> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(message) = decode_text_chat_message(envelope) else {
                continue;
            };
            if sender.unbounded_send(message).is_err() {
                break;
            }
        }
    });

    receiver
}

fn decode_text_chat_message(envelope: RealtimeEnvelope) -> Option<TextChatMessage> {
    if envelope.module != RealtimeModule::TextChat
        || envelope.kind != RealtimeKind::TextChat(TextChatKind::MessageCreated)
    {
        return None;
    }

    serde_json::from_value::<TextChatMessage>(envelope.payload).ok()
}
