//! Text chat infrastructure layer.

mod entities;
mod in_memory;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::features::text_chat::domain::TextMessage;
use crate::features::text_chat::infrastructure::entities::text_messages;

pub(crate) use in_memory::InMemoryTextChatStore;

const HISTORY_LIMIT: u64 = 50;

/// Text chat storage boundary.
#[async_trait]
pub(crate) trait TextChatStore: Send + Sync {
    /// Inserts a prebuilt text message.
    async fn insert_text_message(&self, message: TextMessage) -> anyhow::Result<()>;

    /// Loads the latest room messages, oldest-to-newest.
    async fn latest_room_messages(&self, room_id: &Uuid) -> anyhow::Result<Vec<TextMessage>>;
}

/// Postgres-backed text chat storage.
pub(crate) struct PostgresTextChatStore {
    database: DatabaseConnection,
}

impl PostgresTextChatStore {
    /// Builds a Postgres-backed text chat storage.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl TextChatStore for PostgresTextChatStore {
    async fn insert_text_message(&self, message: TextMessage) -> anyhow::Result<()> {
        insert_text_message(&self.database, message).await
    }

    async fn latest_room_messages(&self, room_id: &Uuid) -> anyhow::Result<Vec<TextMessage>> {
        latest_room_messages(&self.database, room_id).await
    }
}

async fn insert_text_message(
    database: &impl ConnectionTrait,
    message: TextMessage,
) -> anyhow::Result<()> {
    text_messages::ActiveModel {
        id: Set(message.id),
        server_id: Set(message.server_id),
        room_id: Set(message.room_id),
        author_user_id: Set(message.author_user_id),
        author_nickname: Set(message.author_nickname),
        body: Set(message.body),
        created_at: Set(message.created_at),
    }
    .insert(database)
    .await?;

    Ok(())
}

async fn latest_room_messages(
    database: &impl ConnectionTrait,
    room_id: &Uuid,
) -> anyhow::Result<Vec<TextMessage>> {
    let mut messages = text_messages::Entity::find()
        .filter(text_messages::Column::RoomId.eq(*room_id))
        .order_by_desc(text_messages::Column::CreatedAt)
        .order_by_desc(text_messages::Column::Id)
        .limit(HISTORY_LIMIT)
        .all(database)
        .await?
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();

    messages.sort_by_key(|message: &TextMessage| (message.created_at, message.id));

    Ok(messages)
}

impl From<text_messages::Model> for TextMessage {
    fn from(row: text_messages::Model) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            room_id: row.room_id,
            author_user_id: row.author_user_id,
            author_nickname: row.author_nickname,
            body: row.body,
            created_at: row.created_at,
        }
    }
}
