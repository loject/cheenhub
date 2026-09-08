//! Окончательное обезличивание просроченных tombstone.
use super::entities::{
    oauth_accounts, oauth_handoffs, oauth_states, password_reset_tokens, sessions,
    user_nickname_history, user_password_change_trace, users,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, Set, TransactionTrait, sea_query::LockType,
};

pub(super) async fn finalize_expired_account_deletions(
    db: &DatabaseConnection,
    now: DateTime<Utc>,
) -> anyhow::Result<u64> {
    let pending = users::Entity::find()
        .filter(users::Column::DeletionRequestedAt.is_not_null())
        .filter(users::Column::DeletionRestoreUntil.lte(now))
        .filter(users::Column::DeletionFinalizedAt.is_null())
        .all(db)
        .await?;
    let mut count = 0;
    for pending in pending {
        let tx = db.begin().await?;
        let Some(user) = users::Entity::find_by_id(pending.id)
            .lock(LockType::Update)
            .one(&tx)
            .await?
        else {
            continue;
        };
        if user.deletion_requested_at.is_none()
            || user
                .deletion_restore_until
                .is_none_or(|deadline| deadline > now)
            || user.deletion_finalized_at.is_some()
        {
            continue;
        }
        let mut user = user.into_active_model();
        user.nickname = Set(format!(
            "deleted:{}",
            URL_SAFE_NO_PAD.encode(pending.id.as_bytes())
        ));
        user.email = Set(format!("deleted:{}", pending.id));
        user.email_normalized = user.email.clone();
        user.password_hash = Set(None);
        user.avatar_image_id = Set(None);
        user.updated_at = Set(now);
        user.deletion_token_hash = Set(None);
        user.deletion_finalized_at = Set(Some(now));
        user.update(&tx).await?;
        oauth_accounts::Entity::delete_many()
            .filter(oauth_accounts::Column::UserId.eq(pending.id))
            .exec(&tx)
            .await?;
        oauth_handoffs::Entity::delete_many()
            .filter(oauth_handoffs::Column::UserId.eq(pending.id))
            .exec(&tx)
            .await?;
        oauth_states::Entity::delete_many()
            .filter(oauth_states::Column::UserId.eq(pending.id))
            .exec(&tx)
            .await?;
        password_reset_tokens::Entity::delete_many()
            .filter(password_reset_tokens::Column::UserId.eq(pending.id))
            .exec(&tx)
            .await?;
        user_nickname_history::Entity::delete_many()
            .filter(user_nickname_history::Column::UserId.eq(pending.id))
            .exec(&tx)
            .await?;
        user_password_change_trace::Entity::delete_many()
            .filter(user_password_change_trace::Column::UserId.eq(pending.id))
            .exec(&tx)
            .await?;
        sessions::Entity::delete_many()
            .filter(sessions::Column::UserId.eq(pending.id))
            .exec(&tx)
            .await?;
        tx.commit().await?;
        count += 1;
        tracing::info!(user_id = %pending.id, "expired account tombstone anonymized");
    }
    Ok(count)
}
