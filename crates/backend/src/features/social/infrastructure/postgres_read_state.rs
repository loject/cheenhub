//! Вспомогательные операции Postgres read-state личных сообщений.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set,
    sea_query::{Expr, ExprTrait, Func},
};
use uuid::Uuid;

use crate::features::social::infrastructure::entities::conversation_member_states;

/// Возвращает существующий read-state участника или создает пустой.
pub(super) async fn ensure_member_state<C>(
    database: &C,
    conversation_id: &Uuid,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<conversation_member_states::Model>
where
    C: ConnectionTrait,
{
    if let Some(row) = conversation_member_states::Entity::find()
        .filter(conversation_member_states::Column::ConversationId.eq(*conversation_id))
        .filter(conversation_member_states::Column::UserId.eq(*user_id))
        .one(database)
        .await?
    {
        return Ok(row);
    }
    Ok(conversation_member_states::ActiveModel {
        conversation_id: Set(*conversation_id),
        user_id: Set(*user_id),
        last_read_message_id: Set(None),
        last_read_seq: Set(0),
        last_read_at: Set(None),
        unread_count: Set(0),
        updated_at: Set(now),
    }
    .insert(database)
    .await?)
}

/// Увеличивает счетчик непрочитанных для получателя входящего сообщения.
pub(super) async fn increment_unread<C>(
    database: &C,
    conversation_id: &Uuid,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<()>
where
    C: ConnectionTrait,
{
    ensure_member_state(database, conversation_id, user_id, now).await?;
    conversation_member_states::Entity::update_many()
        .col_expr(
            conversation_member_states::Column::UnreadCount,
            Func::greatest([
                Expr::col(conversation_member_states::Column::UnreadCount).into(),
                Expr::value(0),
            ])
            .add(1),
        )
        .col_expr(
            conversation_member_states::Column::UpdatedAt,
            Expr::value(now),
        )
        .filter(conversation_member_states::Column::ConversationId.eq(*conversation_id))
        .filter(conversation_member_states::Column::UserId.eq(*user_id))
        .exec(database)
        .await?;
    Ok(())
}
