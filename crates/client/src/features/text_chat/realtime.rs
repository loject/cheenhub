//! Text chat realtime helpers.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cheenhub_contracts::realtime::{
    AuthorRolesResolved, ChatImageLoadedResponse, ChatImageUploadResponse, DeleteMessage,
    DeleteMessageAccepted, EditMessage, EditMessageAccepted, LoadChatImage, LoadRoomHistory,
    MessageDeletedPayload, RealtimeEnvelope, RealtimeKind, RealtimeModule, ResolveAuthorRoles,
    RoomHistory, SendMessage, SendMessageAccepted, TextChatKind, TextChatMessage, UploadChatImage,
};
use futures_channel::mpsc;
use futures_util::StreamExt;
use uuid::Uuid;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Входящее событие сообщений комнаты из общей realtime-подписки.
pub(crate) enum TextChatEvent {
    /// В комнате создано новое сообщение.
    Created(TextChatMessage),
    /// Подтверждённое сервером изменение текста сообщения комнаты.
    Edited(TextChatMessage),
    /// Сообщение удалено автором.
    Deleted(MessageDeletedPayload),
}

/// Loads latest text chat history for a room.
pub(crate) async fn load_room_history(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    before_message_id: Option<String>,
) -> Result<RoomHistory, RealtimeError> {
    realtime
        .request_one_shot(
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

/// Запрашивает роли не более 100 авторов, допущенных к чтению комнаты.
pub(crate) async fn resolve_author_roles(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    author_user_ids: Vec<String>,
) -> Result<AuthorRolesResolved, RealtimeError> {
    realtime
        .request_one_shot(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::ResolveAuthorRoles),
            ResolveAuthorRoles {
                server_id,
                room_id,
                author_user_ids,
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
    attachment_id: Option<String>,
) -> Result<SendMessageAccepted, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::SendMessage),
            SendMessage {
                server_id,
                room_id,
                body,
                attachment_ids: attachment_id.into_iter().collect(),
            },
        )
        .await
}

/// Редактирует текст своего сообщения комнаты и ждёт сохранения сервером.
pub(crate) async fn edit_text_message(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    message_id: String,
    body: String,
) -> Result<EditMessageAccepted, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::EditMessage),
            EditMessage {
                server_id,
                room_id,
                message_id,
                body,
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
    attachment_id: Uuid,
) -> Result<ChatImageLoadedResponse, RealtimeError> {
    realtime
        .request_one_shot(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::LoadImage),
            LoadChatImage {
                attachment_id: attachment_id.to_string(),
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

/// Подписывается на создание, редактирование и удаление сообщений комнат.
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
            Some(TextChatEvent::Created(message))
        }
        RealtimeKind::TextChat(TextChatKind::MessageEdited) => {
            let message = serde_json::from_value::<TextChatMessage>(envelope.payload)
                .map_err(|error| {
                    dioxus::prelude::warn!(%error, "invalid text chat MessageEdited payload");
                    error
                })
                .ok()?;
            Some(TextChatEvent::Edited(message))
        }
        RealtimeKind::TextChat(TextChatKind::MessageDeleted) => {
            let payload = serde_json::from_value::<MessageDeletedPayload>(envelope.payload).ok()?;
            Some(TextChatEvent::Deleted(payload))
        }
        _ => None,
    }
}
