//! Text chat realtime helpers.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cheenhub_contracts::realtime::{
    ChatImageLoadedResponse, ChatImageUploadResponse, DeleteMessage, DeleteMessageAccepted,
    LoadChatImage, LoadRoomHistory, MessageDeletedPayload, RealtimeEnvelope, RealtimeKind,
    RealtimeModule, RoomHistory, SendMessage, SendMessageAccepted, TextChatKind, TextChatMessage,
    UploadChatImage,
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
                attachment_ids: Vec::new(),
            },
        )
        .await
}

/// Sends one text chat message with a previously uploaded image attachment.
pub(crate) async fn send_image_message(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    attachment_id: String,
) -> Result<SendMessageAccepted, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::SendMessage),
            SendMessage {
                server_id,
                room_id,
                body: String::new(),
                attachment_ids: vec![attachment_id],
            },
        )
        .await
}

/// Uploads one image attachment over realtime.
pub(crate) async fn upload_chat_image(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    original_filename: Option<String>,
    bytes: Vec<u8>,
) -> Result<ChatImageUploadResponse, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::UploadImage),
            UploadChatImage {
                server_id,
                room_id,
                original_filename,
                data_base64: BASE64.encode(bytes),
            },
        )
        .await
}

/// Loads one chat image attachment over realtime.
pub(crate) async fn load_chat_image(
    realtime: &RealtimeHandle,
    attachment_id: String,
) -> Result<ChatImageLoadedResponse, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::LoadImage),
            LoadChatImage { attachment_id },
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
            let payload = serde_json::from_value::<MessageDeletedPayload>(envelope.payload).ok()?;
            Some(TextChatEvent::MessageDeleted(payload))
        }
        _ => None,
    }
}
