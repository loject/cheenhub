//! Postgres-backed text chat storage.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::features::text_chat::domain::{ChatAttachment, NewChatAttachment, TextMessage};
use crate::features::text_chat::infrastructure::entities::{text_chat_attachments, text_messages};
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
            deleted_by_user_id: Set(None),
        }
        .insert(&self.database)
        .await?;

        let attachment_ids = message
            .attachments
            .iter()
            .map(|attachment| attachment.id)
            .collect::<Vec<_>>();
        if !attachment_ids.is_empty() {
            text_chat_attachments::Entity::update_many()
                .col_expr(
                    text_chat_attachments::Column::MessageId,
                    sea_orm::sea_query::Expr::value(message.id),
                )
                .filter(text_chat_attachments::Column::Id.is_in(attachment_ids))
                .exec(&self.database)
                .await?;
        }

        Ok(())
    }

    async fn insert_chat_attachment(&self, attachment: NewChatAttachment) -> anyhow::Result<()> {
        text_chat_attachments::ActiveModel {
            id: Set(attachment.id),
            server_id: Set(attachment.server_id),
            room_id: Set(attachment.room_id),
            uploader_user_id: Set(attachment.uploader_user_id),
            bucket: Set(attachment.bucket),
            message_id: Set(attachment.message_id),
            object_key: Set(attachment.object_key),
            content_type: Set(attachment.content_type),
            byte_size: Set(attachment.byte_size),
            width: Set(attachment.width),
            height: Set(attachment.height),
            sha256: Set(attachment.sha256),
            original_filename: Set(attachment.original_filename),
            created_at: Set(Utc::now()),
        }
        .insert(&self.database)
        .await?;

        Ok(())
    }

    async fn find_chat_attachment(
        &self,
        attachment_id: &Uuid,
    ) -> anyhow::Result<Option<ChatAttachment>> {
        Ok(text_chat_attachments::Entity::find_by_id(*attachment_id)
            .one(&self.database)
            .await?
            .map(Into::into))
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
        hydrate_attachments(&self.database, &mut messages).await?;

        Ok(TextMessagePage { messages, has_more })
    }

    async fn soft_delete_message(
        &self,
        message_id: &Uuid,
        deleted_by_user_id: &Uuid,
        require_authorship: bool,
    ) -> anyhow::Result<Option<TextMessage>> {
        // NOTE: soft delete необходим для модерации удаленных сообщений в дальнейшем
        let mut query = text_messages::Entity::find()
            .filter(text_messages::Column::Id.eq(*message_id))
            .filter(text_messages::Column::DeletedAt.is_null());
        if require_authorship {
            query = query.filter(text_messages::Column::AuthorUserId.eq(*deleted_by_user_id));
        }
        let Some(row) = query.one(&self.database).await? else {
            return Ok(None);
        };

        let deleted_at = Utc::now();
        let mut active: text_messages::ActiveModel = row.into();
        active.deleted_at = Set(Some(deleted_at));
        active.deleted_by_user_id = Set(Some(*deleted_by_user_id));
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
            attachments: Vec::new(),
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by_user_id: row.deleted_by_user_id,
        }
    }
}

impl From<text_chat_attachments::Model> for ChatAttachment {
    fn from(row: text_chat_attachments::Model) -> Self {
        Self {
            id: row.id,
            server_id: row.server_id,
            room_id: row.room_id,
            uploader_user_id: row.uploader_user_id,
            message_id: row.message_id,
            bucket: row.bucket,
            object_key: row.object_key,
            content_type: row.content_type,
            byte_size: row.byte_size,
            width: row.width,
            height: row.height,
            sha256: row.sha256,
            original_filename: row.original_filename,
            created_at: row.created_at,
        }
    }
}

async fn hydrate_attachments(
    database: &DatabaseConnection,
    messages: &mut [TextMessage],
) -> anyhow::Result<()> {
    let message_ids = messages
        .iter()
        .map(|message| message.id)
        .collect::<Vec<_>>();
    if message_ids.is_empty() {
        return Ok(());
    }

    let mut by_message_id: HashMap<Uuid, Vec<ChatAttachment>> = HashMap::new();
    for attachment in text_chat_attachments::Entity::find()
        .filter(text_chat_attachments::Column::MessageId.is_in(message_ids))
        .order_by_asc(text_chat_attachments::Column::CreatedAt)
        .all(database)
        .await?
        .into_iter()
        .map(ChatAttachment::from)
    {
        if let Some(message_id) = attachment.message_id {
            by_message_id
                .entry(message_id)
                .or_default()
                .push(attachment);
        }
    }

    for message in messages {
        message.attachments = by_message_id.remove(&message.id).unwrap_or_default();
    }

    Ok(())
}
