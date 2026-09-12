//! PostgreSQL-операции OAuth-передачи и регистрации.

use crate::features::auth::domain::{OAuthHandoff, OAuthRegistrationIntent};
use crate::features::auth::infrastructure::entities::{oauth_handoffs, oauth_registration_intents};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
};
use uuid::Uuid;

pub(super) async fn insert(
    database: &DatabaseConnection,
    code_hash: String,
    kind: String,
    user_id: Option<Uuid>,
    registration_intent_id: Option<Uuid>,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let transaction = database.begin().await?;
    if let Some(user_id) = user_id {
        anyhow::ensure!(
            super::postgres_deletion::lock_active_user(&transaction, &user_id).await?,
            "account is deleted or missing"
        );
    }
    oauth_handoffs::ActiveModel {
        id: Set(Uuid::new_v4()),
        code_hash: Set(code_hash),
        kind: Set(kind),
        user_id: Set(user_id),
        registration_intent_id: Set(registration_intent_id),
        created_at: Set(now),
        expires_at: Set(expires_at),
        consumed_at: Set(None),
    }
    .insert(&transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}
pub(super) async fn find_active(
    database: &DatabaseConnection,
    code_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<OAuthHandoff>> {
    Ok(oauth_handoffs::Entity::find()
        .filter(oauth_handoffs::Column::CodeHash.eq(code_hash))
        .filter(oauth_handoffs::Column::ConsumedAt.is_null())
        .filter(oauth_handoffs::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
        .map(Into::into))
}
pub(super) async fn consume(
    database: &DatabaseConnection,
    handoff_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    super::postgres_desktop_oauth::consume_handoff(database, handoff_id, now).await
}
pub(super) async fn insert_registration_intent(
    database: &DatabaseConnection,
    provider: String,
    provider_subject: String,
    email: String,
    display_name: Option<String>,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<OAuthRegistrationIntent> {
    Ok(oauth_registration_intents::ActiveModel {
        id: Set(Uuid::new_v4()),
        provider: Set(provider),
        provider_subject: Set(provider_subject),
        email: Set(email),
        display_name: Set(display_name),
        created_at: Set(now),
        expires_at: Set(expires_at),
        consumed_at: Set(None),
    }
    .insert(database)
    .await?
    .into())
}
pub(super) async fn find_active_registration_intent(
    database: &DatabaseConnection,
    intent_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<OAuthRegistrationIntent>> {
    Ok(oauth_registration_intents::Entity::find_by_id(*intent_id)
        .filter(oauth_registration_intents::Column::ConsumedAt.is_null())
        .filter(oauth_registration_intents::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
        .map(Into::into))
}
pub(super) async fn consume_registration_intent(
    database: &DatabaseConnection,
    intent_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    if let Some(intent) = oauth_registration_intents::Entity::find_by_id(*intent_id)
        .one(database)
        .await?
    {
        let mut intent = intent.into_active_model();
        intent.consumed_at = Set(Some(now));
        intent.update(database).await?;
    }
    Ok(())
}
