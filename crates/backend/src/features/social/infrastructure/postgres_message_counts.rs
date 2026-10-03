//! Счётчики личных сообщений в PostgreSQL.
//!
//! Агрегация по усечённой минуте выполняется на стороне базы: за сутки в
//! приложение попадает не больше 1440 строк вместо всех сообщений окна.

use chrono::{DateTime, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::features::social::domain::DmMessagesPerMinute;
use crate::features::social::infrastructure::entities::dm_messages;

/// Считает все личные сообщения, включая мягко удалённые.
pub(super) async fn count_dm_messages(database: &DatabaseConnection) -> anyhow::Result<u64> {
    Ok(dm_messages::Entity::find().count(database).await?)
}

/// Возвращает число личных сообщений по минутам в полуинтервале `(since, until]`.
pub(super) async fn count_dm_messages_per_minute(
    database: &DatabaseConnection,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> anyhow::Result<Vec<DmMessagesPerMinute>> {
    let minute = Expr::cust(r#"date_trunc('minute', "dm_messages"."created_at")"#);
    let rows = dm_messages::Entity::find()
        .select_only()
        .column_as(minute.clone(), "minute")
        .column_as(dm_messages::Column::Id.count(), "messages")
        .filter(dm_messages::Column::CreatedAt.gt(since))
        .filter(dm_messages::Column::CreatedAt.lte(until))
        .group_by(minute.clone())
        .order_by_asc(minute)
        .into_tuple::<(DateTime<Utc>, i64)>()
        .all(database)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(minute, messages)| DmMessagesPerMinute {
            minute,
            messages: u64::try_from(messages).unwrap_or_default(),
        })
        .collect())
}
