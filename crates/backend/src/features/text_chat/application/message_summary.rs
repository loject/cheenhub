//! Преобразование сохранённого сообщения в realtime-контракт текстового чата.

use cheenhub_contracts::realtime::{TextChatImageAttachment, TextChatMessage};

use crate::features::text_chat::domain::TextMessage;

/// Создаёт публичный снимок сообщения без дополнительных серверных метаданных автора.
///
/// Цвет роли заполняется вызывающим потоком после проверки доступа и загрузки ролей.
pub(super) fn message_summary(
    message: &TextMessage,
    author_avatar_url: Option<String>,
) -> TextChatMessage {
    TextChatMessage {
        id: message.id.to_string(),
        server_id: message.server_id.to_string(),
        room_id: message.room_id.to_string(),
        author_user_id: message.author_user_id.to_string(),
        author_nickname: message.author_nickname.clone(),
        author_avatar_url,
        body: message.body.clone(),
        attachments: message
            .attachments
            .iter()
            .map(|attachment| TextChatImageAttachment {
                id: attachment.id.to_string(),
                content_type: attachment.content_type.clone(),
                byte_size: attachment.byte_size,
                width: attachment.width,
                height: attachment.height,
            })
            .collect(),
        delivery_status: None,
        created_at: message.created_at.to_rfc3339(),
        edited_at: message.edited_at.as_ref().map(|value| value.to_rfc3339()),
    }
}
