//! Удаление истёкших desktop-попыток и принадлежащих им временных OAuth-записей.

use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use super::entities::{desktop_oauth_attempts as attempts, oauth_handoffs, oauth_states};

#[cfg(test)]
#[path = "postgres_desktop_oauth_cleanup_tests.rs"]
mod tests;

/// Удаляет ограниченную порцию истёкших попыток, не затрагивая браузерные потоки.
pub(super) async fn cleanup(
    database: &DatabaseConnection,
    now: DateTime<Utc>,
) -> anyhow::Result<u64> {
    let transaction = database.begin().await?;
    let expired = attempts::Entity::find()
        .filter(attempts::Column::ExpiresAt.lte(now))
        .order_by_asc(attempts::Column::ExpiresAt)
        .order_by_asc(attempts::Column::Id)
        .limit(500)
        .lock_exclusive()
        .all(&transaction)
        .await?;
    if expired.is_empty() {
        transaction.commit().await?;
        return Ok(0);
    }
    // Удаление state каскадно удаляет попытку и личность. Живой state не остаётся
    // без desktop-маркера, поэтому поздний callback не превратится в браузерный вход.
    oauth_states::Entity::delete_many()
        .filter(oauth_states::Column::Id.is_in(expired.iter().map(|row| row.oauth_state_id)))
        .exec(&transaction)
        .await?;
    let handoff_ids: Vec<_> = expired.iter().filter_map(|row| row.handoff_id).collect();
    if !handoff_ids.is_empty() {
        oauth_handoffs::Entity::delete_many()
            .filter(oauth_handoffs::Column::Id.is_in(handoff_ids))
            .exec(&transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(expired.len() as u64)
}
