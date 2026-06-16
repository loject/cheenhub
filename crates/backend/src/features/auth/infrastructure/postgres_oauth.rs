//! Postgres OAuth storage helpers.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::features::auth::domain::{
    OAuthAccount, OAuthHandoff, OAuthRegistrationIntent, OAuthState,
};
use crate::features::auth::infrastructure::entities::{
    oauth_accounts, oauth_handoffs, oauth_registration_intents, oauth_states,
};

pub(super) async fn insert_oauth_state(
    database: &DatabaseConnection,
    state_hash: String,
    nonce: String,
    flow_kind: String,
    user_id: Option<Uuid>,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    oauth_states::ActiveModel {
        id: Set(Uuid::new_v4()),
        state_hash: Set(state_hash),
        nonce: Set(nonce),
        flow_kind: Set(flow_kind),
        user_id: Set(user_id),
        created_at: Set(now),
        expires_at: Set(expires_at),
        consumed_at: Set(None),
    }
    .insert(database)
    .await?;

    Ok(())
}

pub(super) async fn consume_oauth_state(
    database: &DatabaseConnection,
    state_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<OAuthState>> {
    let Some(state) = oauth_states::Entity::find()
        .filter(oauth_states::Column::StateHash.eq(state_hash))
        .filter(oauth_states::Column::ConsumedAt.is_null())
        .filter(oauth_states::Column::ExpiresAt.gt(now))
        .one(database)
        .await?
    else {
        return Ok(None);
    };

    let result = OAuthState {
        nonce: state.nonce.clone(),
        flow_kind: state.flow_kind.clone(),
        user_id: state.user_id,
    };
    let mut active = state.into_active_model();
    active.consumed_at = Set(Some(now));
    active.update(database).await?;

    Ok(Some(result))
}

pub(super) async fn find_oauth_account_by_subject(
    database: &DatabaseConnection,
    provider: &str,
    provider_subject: &str,
) -> anyhow::Result<Option<OAuthAccount>> {
    Ok(oauth_accounts::Entity::find()
        .filter(oauth_accounts::Column::Provider.eq(provider))
        .filter(oauth_accounts::Column::ProviderSubject.eq(provider_subject))
        .one(database)
        .await?
        .map(Into::into))
}

pub(super) async fn find_oauth_account_for_user(
    database: &DatabaseConnection,
    provider: &str,
    user_id: &Uuid,
) -> anyhow::Result<Option<OAuthAccount>> {
    Ok(oauth_accounts::Entity::find()
        .filter(oauth_accounts::Column::Provider.eq(provider))
        .filter(oauth_accounts::Column::UserId.eq(*user_id))
        .one(database)
        .await?
        .map(Into::into))
}

pub(super) async fn list_oauth_accounts(
    database: &DatabaseConnection,
    user_id: &Uuid,
) -> anyhow::Result<Vec<OAuthAccount>> {
    Ok(oauth_accounts::Entity::find()
        .filter(oauth_accounts::Column::UserId.eq(*user_id))
        .all(database)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

pub(super) async fn insert_oauth_account(
    database: &DatabaseConnection,
    user_id: &Uuid,
    provider: String,
    provider_subject: String,
    email: String,
    display_name: Option<String>,
    now: DateTime<Utc>,
) -> anyhow::Result<OAuthAccount> {
    Ok(oauth_accounts::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(*user_id),
        provider: Set(provider),
        provider_subject: Set(provider_subject),
        email: Set(email),
        display_name: Set(display_name),
        linked_at: Set(now),
    }
    .insert(database)
    .await?
    .into())
}

pub(super) async fn delete_oauth_account(
    database: &DatabaseConnection,
    provider: &str,
    user_id: &Uuid,
) -> anyhow::Result<bool> {
    let result = oauth_accounts::Entity::delete_many()
        .filter(oauth_accounts::Column::Provider.eq(provider))
        .filter(oauth_accounts::Column::UserId.eq(*user_id))
        .exec(database)
        .await?;

    Ok(result.rows_affected > 0)
}

pub(super) async fn insert_oauth_handoff(
    database: &DatabaseConnection,
    code_hash: String,
    kind: String,
    user_id: Option<Uuid>,
    registration_intent_id: Option<Uuid>,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<()> {
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
    .insert(database)
    .await?;

    Ok(())
}

pub(super) async fn find_active_oauth_handoff(
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

pub(super) async fn consume_oauth_handoff(
    database: &DatabaseConnection,
    handoff_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    if let Some(handoff) = oauth_handoffs::Entity::find_by_id(*handoff_id)
        .one(database)
        .await?
    {
        let mut handoff = handoff.into_active_model();
        handoff.consumed_at = Set(Some(now));
        handoff.update(database).await?;
    }

    Ok(())
}

pub(super) async fn insert_oauth_registration_intent(
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

pub(super) async fn find_active_oauth_registration_intent(
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

pub(super) async fn consume_oauth_registration_intent(
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
