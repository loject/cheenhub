//! Text chat infrastructure layer.

mod entities;
mod in_memory;
mod object_storage;
mod postgres;

use async_trait::async_trait;
use uuid::Uuid;

use crate::features::text_chat::domain::{ChatAttachment, NewChatAttachment, TextMessage};

pub(crate) use in_memory::InMemoryTextChatStore;
#[cfg(test)]
pub(crate) use object_storage::InMemoryChatAttachmentObjectStore;
pub(crate) use object_storage::{
    ChatAttachmentObjectStore, DisabledChatAttachmentObjectStore, S3ChatAttachmentObjectStore,
};
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

    /// Inserts chat attachment metadata.
    async fn insert_chat_attachment(&self, attachment: NewChatAttachment) -> anyhow::Result<()>;

    /// Finds chat attachment metadata by id.
    async fn find_chat_attachment(
        &self,
        attachment_id: &Uuid,
    ) -> anyhow::Result<Option<ChatAttachment>>;

    /// Loads one room message page, oldest-to-newest.
    async fn room_message_page(
        &self,
        room_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<TextMessagePage>;

    /// Soft-deletes a message, recording who deleted it.
    ///
    /// When `require_authorship` is `true` the delete is rejected unless
    /// `deleted_by_user_id` matches the message author (own-message delete).
    /// When `false`, any non-deleted message is deleted regardless of author
    /// (moderation delete).
    ///
    /// Returns `Some(updated_message)` on success, `None` when the message
    /// does not exist, is already deleted, or authorship check fails.
    async fn soft_delete_message(
        &self,
        message_id: &Uuid,
        deleted_by_user_id: &Uuid,
        require_authorship: bool,
    ) -> anyhow::Result<Option<TextMessage>>;
}
