//! Postgres-backed text chat storage.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::features::text_chat::domain::TextMessage;
use crate::features::text_chat::infrastructure::entities::text_messages;
use crate::features::text_chat::infrastructure::{HISTORY_LIMIT, TextChatStore, TextMessagePage};

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
        text_messages::ActiveModel {
            id: Set(message.id),
            server_id: Set(message.server_id),
            room_id: Set(message.room_id),
            author_user_id: Set(message.author_user_id),
            author_nickname: Set(message.author_nickname),
            body: Set(message.body),
            created_at: Set(message.created_at),
            deleted_at: Set(None),
        }
        .insert(&self.database)
        .await?;

        Ok(())
    }

    async fn room_message_page(
        &self,
        room_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<TextMessagePage> {
        let before_message = match before_message_id {
            Some(message_id) => Some(
                text_messages::Entity::find()
                    .filter(text_messages::Column::RoomId.eq(*room_id))
                    .filter(text_messages::Column::Id.eq(*message_id))
                    .one(&self.database)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("message history cursor was not found"))?,
            ),
            None => None,
        };
        let mut filter = Condition::all()
            .add(text_messages::Column::RoomId.eq(*room_id))
            .add(text_messages::Column::DeletedAt.is_null());

        if let Some(message) = before_message {
            filter = filter.add(
                Condition::any()
                    .add(text_messages::Column::CreatedAt.lt(message.created_at))
                    .add(
                        Condition::all()
                            .add(text_messages::Column::CreatedAt.eq(message.created_at))
                            .add(text_messages::Column::Id.lt(message.id)),
                    ),
            );
        }

        let mut messages = text_messages::Entity::find()
            .filter(filter)
            .order_by_desc(text_messages::Column::CreatedAt)
            .order_by_desc(text_messages::Column::Id)
            .limit(HISTORY_LIMIT + 1)
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let has_more = messages.len() > usize::try_from(HISTORY_LIMIT).unwrap_or(50);

        if has_more {
            messages.truncate(usize::try_from(HISTORY_LIMIT).unwrap_or(50));
        }

        messages.sort_by_key(|message: &TextMessage| (message.created_at, message.id));

        Ok(TextMessagePage { messages, has_more })
    }

    async fn soft_delete_message(
        &self,
        message_id: &Uuid,
        author_user_id: &Uuid,
    ) -> anyhow::Result<Option<TextMessage>> {
        // NOTE: soft delete необходим для модерации удаленных сообщений в дальнейшем
        let Some(row) = text_messages::Entity::find()
            .filter(text_messages::Column::Id.eq(*message_id))
            .filter(text_messages::Column::AuthorUserId.eq(*author_user_id))
            .filter(text_messages::Column::DeletedAt.is_null())
            .one(&self.database)
            .await?
        else {
            return Ok(None);
        };

        let deleted_at = Utc::now();
        let mut active: text_messages::ActiveModel = row.into();
        active.deleted_at = Set(Some(deleted_at));
        let updated = active.update(&self.database).await?;

        Ok(Some(updated.into()))
    }
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
            deleted_at: row.deleted_at,
        }
    }
}
