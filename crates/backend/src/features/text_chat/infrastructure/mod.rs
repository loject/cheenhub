//! Text chat infrastructure layer.

mod entities;
mod in_memory;
mod postgres;

use async_trait::async_trait;
use uuid::Uuid;

use crate::features::text_chat::domain::TextMessage;

pub(crate) use in_memory::InMemoryTextChatStore;
pub(crate) use postgres::PostgresTextChatStore;

const HISTORY_LIMIT: u64 = 50;

/// One page of text messages.
pub(crate) struct TextMessagePage {
    /// Messages in oldest-to-newest order.
    pub(crate) messages: Vec<TextMessage>,
    /// Whether older messages are available before this page.
    pub(crate) has_more: bool,
}

/// Text chat storage boundary.
#[async_trait]
pub(crate) trait TextChatStore: Send + Sync {
    /// Inserts a prebuilt text message.
    async fn insert_text_message(&self, message: TextMessage) -> anyhow::Result<()>;

    /// Loads one room message page, oldest-to-newest.
    async fn room_message_page(
        &self,
        room_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<TextMessagePage>;

    /// Soft-deletes a message owned by `author_user_id`.
    ///
    /// Returns `Some(updated_message)` when the message was found and deleted,
    /// or `None` when it does not exist, belongs to another user, or was already deleted.
    async fn soft_delete_message(
        &self,
        message_id: &Uuid,
        author_user_id: &Uuid,
    ) -> anyhow::Result<Option<TextMessage>>;
}
