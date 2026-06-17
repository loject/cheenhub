//! Простое in-memory-хранилище текстового чата.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::features::text_chat::domain::{ChatAttachment, NewChatAttachment, TextMessage};
use crate::features::text_chat::infrastructure::{HISTORY_LIMIT, TextChatStore, TextMessagePage};

/// In-memory-хранилище текстового чата для локального запуска и тестов.
#[derive(Default)]
pub(crate) struct InMemoryTextChatStore {
    messages: Mutex<Vec<TextMessage>>,
    attachments: Mutex<Vec<ChatAttachment>>,
}

#[async_trait]
impl TextChatStore for InMemoryTextChatStore {
    async fn insert_text_message(&self, message: TextMessage) -> anyhow::Result<()> {
        let attachment_ids = message
            .attachments
            .iter()
            .map(|attachment| attachment.id)
            .collect::<Vec<_>>();
        {
            let mut attachments = self.attachments.lock().map_err(|_| poisoned())?;
            for attachment in attachments
                .iter_mut()
                .filter(|attachment| attachment_ids.contains(&attachment.id))
            {
                attachment.message_id = Some(message.id);
            }
        }
        self.messages.lock().map_err(|_| poisoned())?.push(message);

        Ok(())
    }

    async fn insert_chat_attachment(&self, attachment: NewChatAttachment) -> anyhow::Result<()> {
        self.attachments
            .lock()
            .map_err(|_| poisoned())?
            .push(ChatAttachment {
                id: attachment.id,
                server_id: attachment.server_id,
                room_id: attachment.room_id,
                uploader_user_id: attachment.uploader_user_id,
                message_id: attachment.message_id,
                bucket: attachment.bucket,
                object_key: attachment.object_key,
                content_type: attachment.content_type,
                byte_size: attachment.byte_size,
                width: attachment.width,
                height: attachment.height,
                sha256: attachment.sha256,
                original_filename: attachment.original_filename,
                created_at: Utc::now(),
            });

        Ok(())
    }

    async fn find_chat_attachment(
        &self,
        attachment_id: &Uuid,
    ) -> anyhow::Result<Option<ChatAttachment>> {
        Ok(self
            .attachments
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .find(|attachment| attachment.id == *attachment_id)
            .cloned())
    }

    async fn room_message_page(
        &self,
        room_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<TextMessagePage> {
        let mut messages = self
            .messages
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|message| message.room_id == *room_id && message.deleted_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let attachments = self.attachments.lock().map_err(|_| poisoned())?.clone();
        for message in &mut messages {
            message.attachments = attachments
                .iter()
                .filter(|attachment| attachment.message_id == Some(message.id))
                .cloned()
                .collect();
        }
        messages.sort_by_key(|message| (message.created_at, message.id));
        if let Some(before_message_id) = before_message_id {
            let Some(cursor_index) = messages
                .iter()
                .position(|message| message.id == *before_message_id)
            else {
                return Err(anyhow!("message history cursor was not found"));
            };
            messages.truncate(cursor_index);
        }
        let start = messages
            .len()
            .saturating_sub(usize::try_from(HISTORY_LIMIT).unwrap_or(50));
        let has_more = start > 0;

        Ok(TextMessagePage {
            messages: messages.split_off(start),
            has_more,
        })
    }

    async fn soft_delete_message(
        &self,
        message_id: &Uuid,
        deleted_by_user_id: &Uuid,
        require_authorship: bool,
    ) -> anyhow::Result<Option<TextMessage>> {
        let mut messages = self.messages.lock().map_err(|_| poisoned())?;
        let Some(message) = messages.iter_mut().find(|m| {
            m.id == *message_id
                && m.deleted_at.is_none()
                && (!require_authorship || m.author_user_id == *deleted_by_user_id)
        }) else {
            return Ok(None);
        };
        message.deleted_at = Some(Utc::now());
        message.deleted_by_user_id = Some(*deleted_by_user_id);

        Ok(Some(message.clone()))
    }
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory text chat store lock poisoned")
}
