//! Атомарное создание и восстановление tombstone аккаунта.
use super::entities::{password_reset_tokens, sessions, users};
use crate::features::auth::domain::AccountDeletion;
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
    sea_query::{Expr, LockType},
};
use uuid::Uuid;

pub(super) async fn account_deletion<C: ConnectionTrait>(
    db: &C,
    user_id: &Uuid,
) -> anyhow::Result<Option<AccountDeletion>> {
    Ok(users::Entity::find_by_id(*user_id)
        .select_only()
        .columns([
            users::Column::DeletionRequestedAt,
            users::Column::DeletionRestoreUntil,
        ])
        .into_tuple::<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)>()
        .one(db)
        .await?
        .and_then(|(requested_at, restore_until)| {
            Some(AccountDeletion {
                requested_at: requested_at?,
                restore_until: restore_until?,
            })
        }))
}

pub(super) async fn lock_active_user<C: ConnectionTrait>(
    db: &C,
    user_id: &Uuid,
) -> anyhow::Result<bool> {
    Ok(users::Entity::find_by_id(*user_id)
        .lock(LockType::Update)
        .one(db)
        .await?
        .is_some_and(|row| row.deletion_requested_at.is_none()))
}

pub(super) async fn begin_account_deletion(
    db: &DatabaseConnection,
    user_id: &Uuid,
    token_hash: String,
    now: DateTime<Utc>,
    restore_until: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let tx = db.begin().await?;
    if !lock_active_user(&tx, user_id).await? {
        return Ok(false);
    }
    users::Entity::update_many()
        .col_expr(users::Column::DeletionRequestedAt, Expr::value(now))
        .col_expr(
            users::Column::DeletionRestoreUntil,
            Expr::value(restore_until),
        )
        .col_expr(users::Column::DeletionTokenHash, Expr::value(token_hash))
        .col_expr(
            users::Column::DeletionFinalizedAt,
            Expr::value(None::<DateTime<Utc>>),
        )
        .filter(users::Column::Id.eq(*user_id))
        .exec(&tx)
        .await?;
    sessions::Entity::update_many()
        .col_expr(sessions::Column::RevokedAt, Expr::value(now))
        .filter(sessions::Column::UserId.eq(*user_id))
        .exec(&tx)
        .await?;
    password_reset_tokens::Entity::update_many()
        .col_expr(password_reset_tokens::Column::ConsumedAt, Expr::value(now))
        .filter(password_reset_tokens::Column::UserId.eq(*user_id))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    tracing::info!(%user_id, %restore_until, "account tombstone created and sessions revoked");
    Ok(true)
}

pub(super) async fn restore_account(
    db: &DatabaseConnection,
    token_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let tx = db.begin().await?;
    let Some(row) = users::Entity::find()
        .filter(users::Column::DeletionTokenHash.eq(token_hash))
        .one(&tx)
        .await?
    else {
        return Ok(false);
    };
    users::Entity::find_by_id(row.id)
        .lock(LockType::Update)
        .one(&tx)
        .await?;
    // Ожидание блокировки не продлевает срок восстановления.
    let now = now.max(Utc::now());
    let restored = users::Entity::update_many()
        .col_expr(
            users::Column::DeletionRequestedAt,
            Expr::value(None::<DateTime<Utc>>),
        )
        .col_expr(
            users::Column::DeletionRestoreUntil,
            Expr::value(None::<DateTime<Utc>>),
        )
        .col_expr(
            users::Column::DeletionTokenHash,
            Expr::value(None::<String>),
        )
        .col_expr(
            users::Column::DeletionFinalizedAt,
            Expr::value(None::<DateTime<Utc>>),
        )
        .filter(users::Column::Id.eq(row.id))
        .filter(users::Column::DeletionTokenHash.eq(token_hash))
        .filter(users::Column::DeletionRestoreUntil.gt(now))
        .filter(users::Column::DeletionFinalizedAt.is_null())
        .exec(&tx)
        .await?;
    tx.commit().await?;
    if restored.rows_affected == 1 {
        tracing::info!(user_id = %row.id, "account restored from tombstone");
    }
    Ok(restored.rows_affected == 1)
}

/// Исключает tombstone из публичного поиска и выдачи аватаров.
pub(super) fn active_users_filter() -> sea_orm::sea_query::SimpleExpr {
    users::Column::DeletionRequestedAt.is_null()
}
