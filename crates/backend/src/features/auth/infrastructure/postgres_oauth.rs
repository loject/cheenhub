//! Вспомогательные функции хранения OAuth для Postgres.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait, sea_query::Expr,
};
use uuid::Uuid;

use crate::features::auth::domain::{OAuthAccount, OAuthState};
use crate::features::auth::infrastructure::entities::{oauth_accounts, oauth_states};

pub(super) async fn insert_oauth_state(
    database: &DatabaseConnection,
    state_hash: String,
    nonce: String,
    flow_kind: String,
    user_id: Option<Uuid>,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<Uuid> {
    let transaction = database.begin().await?;
    if let Some(user_id) = user_id {
        anyhow::ensure!(
            super::postgres_deletion::lock_active_user(&transaction, &user_id).await?,
            "account is deleted or missing"
        );
    }
    let id = Uuid::new_v4();
    oauth_states::ActiveModel {
        id: Set(id),
        state_hash: Set(state_hash),
        nonce: Set(nonce),
        flow_kind: Set(flow_kind),
        user_id: Set(user_id),
        created_at: Set(now),
        expires_at: Set(expires_at),
        consumed_at: Set(None),
    }
    .insert(&transaction)
    .await?;

    transaction.commit().await?;
    Ok(id)
}

pub(super) async fn consume_oauth_state(
    database: &DatabaseConnection,
    state_hash: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<OAuthState>> {
    let Some(state) = oauth_states::Entity::update_many()
        .col_expr(oauth_states::Column::ConsumedAt, Expr::value(now))
        .filter(oauth_states::Column::StateHash.eq(state_hash))
        .filter(oauth_states::Column::ConsumedAt.is_null())
        .filter(oauth_states::Column::ExpiresAt.gt(now))
        .exec_with_returning(database)
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };

    Ok(Some(OAuthState {
        id: state.id,
        nonce: state.nonce,
        flow_kind: state.flow_kind,
        user_id: state.user_id,
    }))
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
    let transaction = database.begin().await?;
    anyhow::ensure!(
        super::postgres_deletion::lock_active_user(&transaction, user_id).await?,
        "account is deleted or missing"
    );
    let account = oauth_accounts::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(*user_id),
        provider: Set(provider),
        provider_subject: Set(provider_subject),
        email: Set(email),
        display_name: Set(display_name),
        linked_at: Set(now),
    }
    .insert(&transaction)
    .await?;
    transaction.commit().await?;
    Ok(account.into())
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
