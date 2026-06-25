//! Инфраструктурный слой текстового чата.

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

/// Одна страница текстовых сообщений.
pub(crate) struct TextMessagePage {
    /// Сообщения в порядке от старых к новым.
    pub(crate) messages: Vec<TextMessage>,
    /// Доступны ли более старые сообщения перед этой страницей.
    pub(crate) has_more: bool,
}

/// Граница хранилища текстового чата.
#[async_trait]
pub(crate) trait TextChatStore: Send + Sync {
    /// Вставляет заранее собранное текстовое сообщение.
    async fn insert_text_message(&self, message: TextMessage) -> anyhow::Result<()>;

    /// Вставляет метаданные вложения чата.
    async fn insert_chat_attachment(&self, attachment: NewChatAttachment) -> anyhow::Result<()>;

    /// Находит метаданные вложения чата по идентификатору.
    async fn find_chat_attachment(
        &self,
        attachment_id: &Uuid,
    ) -> anyhow::Result<Option<ChatAttachment>>;

    /// Загружает одну страницу сообщений комнаты в порядке от старых к новым.
    async fn room_message_page(
        &self,
        room_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<TextMessagePage>;

    /// Мягко удаляет сообщение, фиксируя, кто его удалил.
    ///
    /// Удаление всегда ограничено сообщением, которое реально принадлежит
    /// паре `server_id`/`room_id`. Это не позволяет использовать проверку прав,
    /// выполненную для запрошенной комнаты, чтобы удалить сообщение из чужой
    /// комнаты или сервера (IDOR между арендаторами).
    ///
    /// Когда `require_authorship` равно `true`, удаление отклоняется, если только
    /// `deleted_by_user_id` не совпадает с автором сообщения (удаление своего сообщения).
    /// Когда `false`, удаляется любое не удаленное сообщение независимо от автора
    /// (удаление модератором).
    ///
    /// Возвращает `Some(updated_message)` при успехе, `None`, когда сообщение
    /// не существует, уже удалено, не принадлежит указанной комнате или
    /// проверка авторства не прошла.
    async fn soft_delete_message(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
        message_id: &Uuid,
        deleted_by_user_id: &Uuid,
        require_authorship: bool,
    ) -> anyhow::Result<Option<TextMessage>>;
}
