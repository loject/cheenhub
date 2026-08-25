//! Хранилище глобальных настроек хоста.

pub(crate) mod entities;

use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::domain::{GmailOAuthState, HostEmailSettings};

/// Операции хранения настроек хоста.
#[async_trait]
pub(crate) trait HostSettingsStore: Send + Sync {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool>;
    async fn load_email_settings(&self) -> anyhow::Result<HostEmailSettings>;
    async fn save_email_settings(
        &self,
        settings: HostEmailSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostEmailSettings>;
    async fn insert_gmail_oauth_state(&self, state: GmailOAuthState) -> anyhow::Result<()>;
    async fn consume_gmail_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>>;
}

/// PostgreSQL-хранилище настроек хоста.
pub(crate) struct PostgresHostSettingsStore {
    database: DatabaseConnection,
}

impl PostgresHostSettingsStore {
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl HostSettingsStore for PostgresHostSettingsStore {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool> {
        use entities::host_owners;
        Ok(host_owners::Entity::find_by_id(user_id)
            .one(&self.database)
            .await?
            .is_some())
    }

    async fn load_email_settings(&self) -> anyhow::Result<HostEmailSettings> {
        use entities::host_email_settings;
        let Some(model) = host_email_settings::Entity::find_by_id(super::domain::EMAIL_SETTINGS_ID)
            .one(&self.database)
            .await?
        else {
            return Ok(HostEmailSettings::default());
        };
        settings_from_model(model)
    }

    async fn save_email_settings(
        &self,
        settings: HostEmailSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostEmailSettings> {
        use entities::host_email_settings;
        let active = host_email_settings::ActiveModel {
            id: Set(super::domain::EMAIL_SETTINGS_ID),
            transport: Set(settings.transport.as_str().to_owned()),
            email_send_timeout_seconds: Set(i32::try_from(settings.email_send_timeout_seconds)?),
            smtp_host: Set(settings.smtp_host.clone()),
            smtp_port: Set(Some(i32::from(settings.smtp_port))),
            smtp_username: Set(settings.smtp_username.clone()),
            smtp_password: Set(settings.smtp_password.clone()),
            smtp_from_email: Set(settings.smtp_from_email.clone()),
            gmail_client_id: Set(settings.gmail_client_id.clone()),
            gmail_client_secret: Set(settings.gmail_client_secret.clone()),
            gmail_refresh_token: Set(settings.gmail_refresh_token.clone()),
            gmail_from_email: Set(settings.gmail_from_email.clone()),
            updated_at: Set(updated_at),
            updated_by_user_id: Set(Some(updated_by)),
        };
        if host_email_settings::Entity::find_by_id(super::domain::EMAIL_SETTINGS_ID)
            .one(&self.database)
            .await?
            .is_some()
        {
            active.update(&self.database).await?;
        } else {
            active.insert(&self.database).await?;
        }
        Ok(settings)
    }

    async fn insert_gmail_oauth_state(&self, state: GmailOAuthState) -> anyhow::Result<()> {
        use entities::host_gmail_oauth_states;
        host_gmail_oauth_states::ActiveModel {
            id: Set(state.id),
            state_hash: Set(state.state_hash),
            user_id: Set(state.user_id),
            created_at: Set(state.created_at),
            expires_at: Set(state.expires_at),
            consumed_at: Set(None),
        }
        .insert(&self.database)
        .await?;
        Ok(())
    }

    async fn consume_gmail_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>> {
        use entities::host_gmail_oauth_states;
        let result = host_gmail_oauth_states::Entity::update_many()
            .col_expr(
                host_gmail_oauth_states::Column::ConsumedAt,
                sea_orm::sea_query::Expr::value(now),
            )
            .filter(host_gmail_oauth_states::Column::StateHash.eq(state_hash))
            .filter(host_gmail_oauth_states::Column::ConsumedAt.is_null())
            .filter(host_gmail_oauth_states::Column::ExpiresAt.gt(now))
            .exec(&self.database)
            .await?;
        if result.rows_affected != 1 {
            return Ok(None);
        }
        Ok(host_gmail_oauth_states::Entity::find()
            .filter(host_gmail_oauth_states::Column::StateHash.eq(state_hash))
            .one(&self.database)
            .await?
            .map(|state| state.user_id))
    }
}

fn settings_from_model(
    model: entities::host_email_settings::Model,
) -> anyhow::Result<HostEmailSettings> {
    Ok(HostEmailSettings {
        transport: super::domain::EmailTransport::parse(&model.transport)?,
        email_send_timeout_seconds: u64::try_from(model.email_send_timeout_seconds)?,
        smtp_host: model.smtp_host,
        smtp_port: model
            .smtp_port
            .map(u16::try_from)
            .transpose()?
            .unwrap_or(587),
        smtp_username: model.smtp_username,
        smtp_password: model.smtp_password,
        smtp_from_email: model.smtp_from_email,
        gmail_client_id: model.gmail_client_id,
        gmail_client_secret: model.gmail_client_secret,
        gmail_refresh_token: model.gmail_refresh_token,
        gmail_from_email: model.gmail_from_email,
    })
}

/// In-memory реализация для локальной разработки и тестов.
#[derive(Default)]
pub(crate) struct InMemoryHostSettingsStore {
    settings: RwLock<HostEmailSettings>,
    owners: RwLock<Vec<Uuid>>,
    states: RwLock<Vec<(GmailOAuthState, Option<DateTime<Utc>>)>>,
}

impl InMemoryHostSettingsStore {
    #[cfg(test)]
    pub(crate) fn with_owner(user_id: Uuid) -> Self {
        Self {
            owners: RwLock::new(vec![user_id]),
            ..Self::default()
        }
    }
}

#[async_trait]
impl HostSettingsStore for InMemoryHostSettingsStore {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool> {
        Ok(self
            .owners
            .read()
            .expect("host owners lock")
            .contains(&user_id))
    }

    async fn load_email_settings(&self) -> anyhow::Result<HostEmailSettings> {
        Ok(self.settings.read().expect("host settings lock").clone())
    }

    async fn save_email_settings(
        &self,
        settings: HostEmailSettings,
        _updated_by: Uuid,
        _updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostEmailSettings> {
        *self.settings.write().expect("host settings lock") = settings.clone();
        Ok(settings)
    }

    async fn insert_gmail_oauth_state(&self, state: GmailOAuthState) -> anyhow::Result<()> {
        self.states
            .write()
            .expect("oauth states lock")
            .push((state, None));
        Ok(())
    }

    async fn consume_gmail_oauth_state(
        &self,
        state_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Uuid>> {
        let mut states = self.states.write().expect("oauth states lock");
        let Some((state, consumed_at)) = states.iter_mut().find(|(state, consumed_at)| {
            state.state_hash == state_hash && consumed_at.is_none() && state.expires_at > now
        }) else {
            return Ok(None);
        };
        *consumed_at = Some(now);
        Ok(Some(state.user_id))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::{HostSettingsStore, InMemoryHostSettingsStore};
    use crate::features::host_settings::domain::{
        EmailTransport, GmailOAuthState, HostEmailSettings,
    };

    #[tokio::test]
    async fn distinguishes_host_owner_from_regular_user() {
        let owner_id = Uuid::new_v4();
        let regular_id = Uuid::new_v4();
        let store = InMemoryHostSettingsStore::with_owner(owner_id);

        assert!(store.is_host_owner(owner_id).await.expect("owner lookup"));
        assert!(!store.is_host_owner(regular_id).await.expect("owner lookup"));
    }

    #[tokio::test]
    async fn gmail_oauth_state_is_consumed_only_once() {
        let owner_id = Uuid::new_v4();
        let store = InMemoryHostSettingsStore::with_owner(owner_id);
        let now = Utc::now();
        store
            .insert_gmail_oauth_state(GmailOAuthState {
                id: Uuid::new_v4(),
                state_hash: "hash".to_owned(),
                user_id: owner_id,
                created_at: now,
                expires_at: now + Duration::minutes(10),
            })
            .await
            .expect("state insert");

        assert_eq!(
            store
                .consume_gmail_oauth_state("hash", now)
                .await
                .expect("first consume"),
            Some(owner_id)
        );
        assert_eq!(
            store
                .consume_gmail_oauth_state("hash", now)
                .await
                .expect("second consume"),
            None
        );
    }

    #[tokio::test]
    async fn email_transport_change_is_visible_without_recreating_store() {
        let owner_id = Uuid::new_v4();
        let store = InMemoryHostSettingsStore::with_owner(owner_id);
        assert_eq!(
            store
                .load_email_settings()
                .await
                .expect("initial settings")
                .transport,
            EmailTransport::Smtp
        );

        store
            .save_email_settings(
                HostEmailSettings {
                    transport: EmailTransport::GmailApi,
                    ..HostEmailSettings::default()
                },
                owner_id,
                Utc::now(),
            )
            .await
            .expect("settings update");

        assert_eq!(
            store
                .load_email_settings()
                .await
                .expect("updated settings")
                .transport,
            EmailTransport::GmailApi
        );
    }
}
