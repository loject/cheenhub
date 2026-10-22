//! Операции PostgreSQL для атомарного редактирования сообщений комнат.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use uuid::Uuid;

use super::super::entities::text_messages;
use crate::features::text_chat::domain::TextMessage;

/// Загружает сообщение автора вместе с неизменяемыми вложениями.
pub(super) async fn find_editable_text_message(
    database: &DatabaseConnection,
    server_id: &Uuid,
    room_id: &Uuid,
    message_id: &Uuid,
    author_user_id: &Uuid,
) -> anyhow::Result<Option<TextMessage>> {
    let row = text_messages::Entity::find()
        .filter(text_messages::Column::ServerId.eq(*server_id))
        .filter(text_messages::Column::RoomId.eq(*room_id))
        .filter(text_messages::Column::Id.eq(*message_id))
        .filter(text_messages::Column::AuthorUserId.eq(*author_user_id))
        .filter(text_messages::Column::DeletedAt.is_null())
        .one(database)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut messages = vec![TextMessage::from(row)];
    super::hydrate_attachments(database, &mut messages).await?;
    Ok(messages.pop())
}

/// Блокирует сообщение на время изменения, исключая гонку с удалением.
pub(super) async fn edit_text_message(
    database: &DatabaseConnection,
    server_id: &Uuid,
    room_id: &Uuid,
    message_id: &Uuid,
    author_user_id: &Uuid,
    body: String,
) -> anyhow::Result<Option<TextMessage>> {
    let transaction = database.begin().await?;
    let row = text_messages::Entity::find()
        .filter(text_messages::Column::ServerId.eq(*server_id))
        .filter(text_messages::Column::RoomId.eq(*room_id))
        .filter(text_messages::Column::Id.eq(*message_id))
        .filter(text_messages::Column::AuthorUserId.eq(*author_user_id))
        .filter(text_messages::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&transaction)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut active: text_messages::ActiveModel = row.into();
    active.body = Set(body);
    active.edited_at = Set(Some(Utc::now()));
    let updated = active.update(&transaction).await?;
    let mut messages = vec![TextMessage::from(updated)];
    super::hydrate_attachments(&transaction, &mut messages).await?;
    transaction.commit().await?;
    Ok(messages.pop())
}
