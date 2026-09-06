//! Хранение desktop OAuth с общей блокировкой попытки для отмены и получения результата.

use super::entities::{
    desktop_oauth_attempts::{self as attempts, Column},
    oauth_handoffs, oauth_states,
};
use crate::features::auth::domain::{
    DesktopOAuthAttempt, DesktopOAuthIdentity, DesktopOAuthStatus,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use uuid::Uuid;

/// Создаёт попытку со ссылкой на OAuth state.
pub(super) async fn insert(
    database: &DatabaseConnection,
    attempt: DesktopOAuthAttempt,
) -> anyhow::Result<()> {
    attempts::ActiveModel {
        id: Set(attempt.id),
        oauth_state_id: Set(attempt.oauth_state_id),
        secret_hash: Set(attempt.secret_hash),
        expires_at: Set(attempt.expires_at),
        status: Set("pending".to_owned()),
        handoff_id: Set(None),
        provider_subject: Set(None),
        email: Set(None),
        display_name: Set(None),
        error_message: Set(None),
    }
    .insert(database)
    .await?;
    Ok(())
}

/// Находит попытку по внешнему ключу состояния, в том числе после отмены.
pub(super) async fn find_by_state(
    database: &DatabaseConnection,
    state_hash: &str,
) -> anyhow::Result<Option<Uuid>> {
    let Some(state) = oauth_states::Entity::find()
        .filter(oauth_states::Column::StateHash.eq(state_hash))
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    Ok(attempts::Entity::find()
        .filter(Column::OauthStateId.eq(state.id))
        .one(database)
        .await?
        .map(|a| a.id))
}

/// Читает состояние только после проверки хеша секрета.
pub(super) async fn status(
    database: &DatabaseConnection,
    id: &Uuid,
    secret_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<DesktopOAuthStatus>> {
    let Some(attempt) = attempts::Entity::find_by_id(*id)
        .filter(Column::SecretHash.eq(secret_hash))
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    if attempt.expires_at <= now && matches!(attempt.status.as_str(), "pending" | "ready") {
        return Ok(Some(DesktopOAuthStatus::Expired));
    }
    Ok(Some(match attempt.status.as_str() {
        "pending" => DesktopOAuthStatus::Pending,
        "ready" => DesktopOAuthStatus::Ready,
        "cancelled" => DesktopOAuthStatus::Cancelled,
        "claimed" => DesktopOAuthStatus::Claimed,
        "failed" => DesktopOAuthStatus::Failed(
            attempt
                .error_message
                .unwrap_or_else(|| "Не удалось завершить вход".to_owned()),
        ),
        other => anyhow::bail!("unknown desktop OAuth status: {other}"),
    }))
}

/// Проверяет возможность принять результат провайдера.
pub(super) async fn is_pending(
    database: &DatabaseConnection,
    id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    Ok(attempts::Entity::find_by_id(*id)
        .filter(Column::Status.eq("pending"))
        .filter(Column::ExpiresAt.gt(now))
        .one(database)
        .await?
        .is_some())
}

/// Атомарно сохраняет личность и создаёт handoff без изменения аккаунта пользователя.
pub(super) async fn finish(
    database: &DatabaseConnection,
    id: &Uuid,
    kind: String,
    user_id: Option<Uuid>,
    identity: DesktopOAuthIdentity,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let transaction = database.begin().await?;
    let attempt = attempts::Entity::find_by_id(*id)
        .filter(Column::Status.eq("pending"))
        .filter(Column::ExpiresAt.gt(now))
        .lock_exclusive()
        .one(&transaction)
        .await?;
    let Some(attempt) = attempt else {
        transaction.commit().await?;
        return Ok(false);
    };
    let handoff_id = Uuid::new_v4();
    oauth_handoffs::ActiveModel {
        id: Set(handoff_id),
        code_hash: Set(attempt.secret_hash.clone()),
        kind: Set(kind),
        user_id: Set(user_id),
        registration_intent_id: Set(None),
        created_at: Set(now),
        expires_at: Set(attempt.expires_at),
        consumed_at: Set(None),
    }
    .insert(&transaction)
    .await?;
    let mut attempt = attempt.into_active_model();
    attempt.status = Set("ready".to_owned());
    attempt.handoff_id = Set(Some(handoff_id));
    attempt.provider_subject = Set(Some(identity.subject));
    attempt.email = Set(Some(identity.email));
    attempt.display_name = Set(identity.display_name);
    attempt.update(&transaction).await?;
    transaction.commit().await?;
    Ok(true)
}

/// Читает личность готового handoff; изменение аккаунта требует отдельного атомарного захвата.
pub(super) async fn identity_for_handoff(
    database: &DatabaseConnection,
    handoff_id: &Uuid,
) -> anyhow::Result<Option<DesktopOAuthIdentity>> {
    let Some(attempt) = attempts::Entity::find()
        .filter(Column::HandoffId.eq(*handoff_id))
        .filter(Column::Status.is_in(["ready", "claimed"]))
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(DesktopOAuthIdentity {
        subject: attempt
            .provider_subject
            .ok_or_else(|| anyhow::anyhow!("desktop OAuth identity subject missing"))?,
        email: attempt
            .email
            .ok_or_else(|| anyhow::anyhow!("desktop OAuth identity email missing"))?,
        display_name: attempt.display_name,
    }))
}

/// Записывает ошибку только в действующую ожидающую попытку.
pub(super) async fn fail(
    database: &DatabaseConnection,
    id: &Uuid,
    message: String,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    Ok(attempts::Entity::update_many()
        .col_expr(Column::Status, Expr::value("failed"))
        .col_expr(Column::ErrorMessage, Expr::value(message))
        .filter(Column::Id.eq(*id))
        .filter(Column::Status.eq("pending"))
        .filter(Column::ExpiresAt.gt(now))
        .exec(database)
        .await?
        .rows_affected
        == 1)
}

/// Отменяет попытку и отзывает handoff в одной транзакции.
pub(super) async fn cancel(
    database: &DatabaseConnection,
    id: &Uuid,
    secret_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let transaction = database.begin().await?;
    let attempt = attempts::Entity::find_by_id(*id)
        .filter(Column::SecretHash.eq(secret_hash))
        .filter(Column::Status.is_in(["pending", "ready"]))
        .filter(Column::ExpiresAt.gt(now))
        .lock_exclusive()
        .one(&transaction)
        .await?;
    let Some(attempt) = attempt else {
        transaction.commit().await?;
        return Ok(false);
    };
    if let Some(handoff_id) = attempt.handoff_id {
        oauth_handoffs::Entity::update_many()
            .col_expr(oauth_handoffs::Column::ConsumedAt, Expr::value(now))
            .filter(oauth_handoffs::Column::Id.eq(handoff_id))
            .filter(oauth_handoffs::Column::ConsumedAt.is_null())
            .exec(&transaction)
            .await?;
    }
    let mut attempt = attempt.into_active_model();
    attempt.status = Set("cancelled".to_owned());
    attempt.update(&transaction).await?;
    transaction.commit().await?;
    Ok(true)
}

/// Потребляет handoff, сериализуя desktop-получение с отменой на строке попытки.
pub(super) async fn consume_handoff(
    database: &DatabaseConnection,
    handoff_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let transaction = database.begin().await?;
    let attempt = attempts::Entity::find()
        .filter(Column::HandoffId.eq(*handoff_id))
        .lock_exclusive()
        .one(&transaction)
        .await?;
    if let Some(attempt) = &attempt
        && (attempt.status != "ready" || attempt.expires_at <= now)
    {
        transaction.commit().await?;
        return Ok(false);
    }
    let result = oauth_handoffs::Entity::update_many()
        .col_expr(oauth_handoffs::Column::ConsumedAt, Expr::value(now))
        .filter(oauth_handoffs::Column::Id.eq(*handoff_id))
        .filter(oauth_handoffs::Column::ConsumedAt.is_null())
        .filter(oauth_handoffs::Column::ExpiresAt.gt(now))
        .exec(&transaction)
        .await?;
    if result.rows_affected != 1 {
        transaction.commit().await?;
        return Ok(false);
    }
    if let Some(attempt) = attempt {
        let mut attempt = attempt.into_active_model();
        attempt.status = Set("claimed".to_owned());
        attempt.update(&transaction).await?;
    }
    transaction.commit().await?;
    Ok(true)
}
