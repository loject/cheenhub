//! Simple in-memory text chat storage.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use uuid::Uuid;

use crate::features::text_chat::domain::TextMessage;
use crate::features::text_chat::infrastructure::{HISTORY_LIMIT, TextChatStore};

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

    async fn latest_room_messages(&self, room_id: &Uuid) -> anyhow::Result<Vec<TextMessage>> {
        let mut messages = self
            .messages
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|message| message.room_id == *room_id)
            .cloned()
            .collect::<Vec<_>>();
        messages.sort_by_key(|message| (message.created_at, message.id));
        let start = messages
            .len()
            .saturating_sub(usize::try_from(HISTORY_LIMIT).unwrap_or(50));

        Ok(messages.split_off(start))
    }
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory text chat store lock poisoned")
}
