//! Удаление истёкших desktop OAuth и browser-handoff без затрагивания веб-входа.

use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use super::entities::{desktop_oauth_attempts as attempts, oauth_handoffs, oauth_states};

#[cfg(test)]
mod tests;

/// Удаляет ограниченные порции истёкших desktop и browser-попыток.
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
    let expired_browser = oauth_handoffs::Entity::find()
        .filter(oauth_handoffs::Column::Kind.is_in(["browser_pending", "browser_ready"]))
        .filter(oauth_handoffs::Column::ExpiresAt.lte(now))
        .order_by_asc(oauth_handoffs::Column::ExpiresAt)
        .order_by_asc(oauth_handoffs::Column::Id)
        .limit(500)
        .lock_exclusive()
        .all(&transaction)
        .await?;
    if expired.is_empty() && expired_browser.is_empty() {
        transaction.commit().await?;
        return Ok(0);
    }
    if !expired.is_empty() {
        // Удаление state каскадно удаляет desktop-попытку и проверенную личность.
        // Поздний callback не сможет стать обычным браузерным входом.
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
    }
    if !expired_browser.is_empty() {
        oauth_handoffs::Entity::delete_many()
            .filter(oauth_handoffs::Column::Id.is_in(expired_browser.iter().map(|row| row.id)))
            .exec(&transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok((expired.len() + expired_browser.len()) as u64)
}
