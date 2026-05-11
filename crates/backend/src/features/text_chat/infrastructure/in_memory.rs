//! Simple in-memory text chat storage.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use uuid::Uuid;

use crate::features::text_chat::domain::TextMessage;
use crate::features::text_chat::infrastructure::{HISTORY_LIMIT, TextChatStore, TextMessagePage};

/// In-memory text chat storage for local runs and tests.
#[derive(Default)]
pub(crate) struct InMemoryTextChatStore {
    messages: Mutex<Vec<TextMessage>>,
}

#[async_trait]
impl TextChatStore for InMemoryTextChatStore {
    async fn insert_text_message(&self, message: TextMessage) -> anyhow::Result<()> {
        self.messages.lock().map_err(|_| poisoned())?.push(message);

        Ok(())
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
            .filter(|message| message.room_id == *room_id)
            .cloned()
            .collect::<Vec<_>>();
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
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory text chat store lock poisoned")
}
