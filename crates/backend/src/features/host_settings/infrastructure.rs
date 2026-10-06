//! Хранилище глобальных настроек хоста.
//!
//! Модуль объявляет границу хранения и PostgreSQL-реализацию. In-memory
//! вариант для локальной разработки и тестов лежит в `in_memory`.

pub(crate) mod entities;
mod in_memory;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, sea_query::LockType,
};
use uuid::Uuid;

use super::domain::{
    GmailOAuthState, HostEmailSettings, HostLogSettings, HostOwner, RevokeHostOwnerOutcome,
    VoiceActivitySample,
};

pub(crate) use in_memory::InMemoryHostSettingsStore;

/// Операции хранения настроек хоста.
#[async_trait]
pub(crate) trait HostSettingsStore: Send + Sync {
    async fn is_host_owner(&self, user_id: Uuid) -> anyhow::Result<bool>;
    /// Возвращает всех владельцев хоста в хронологическом порядке выдачи прав.
    async fn load_host_owners(&self) -> anyhow::Result<Vec<HostOwner>>;
    /// Выдаёт права владельца хоста пользователю.
    ///
    /// Повторная выдача уже имеющихся прав обновляет время и автора выдачи,
    /// но не создаёт вторую запись: пользователь остаётся одним владельцем.
    async fn grant_host_owner(&self, owner: HostOwner) -> anyhow::Result<()>;
    /// Атомарно отзывает права, сохраняя хотя бы одного владельца хоста.
    ///
    /// Отсутствие пользователя и отказ последнему владельцу не изменяют данные.
    async fn revoke_host_owner(&self, user_id: Uuid) -> anyhow::Result<RevokeHostOwnerOutcome>;
    async fn load_email_settings(&self) -> anyhow::Result<HostEmailSettings>;
    /// Возвращает сохранённый минимальный уровень журнала.
    ///
    /// Отсутствие строки или пустой уровень означают фильтр, заданный при запуске.
    async fn load_log_settings(&self) -> anyhow::Result<HostLogSettings>;
    /// Сохраняет минимальный уровень журнала и отметку времени изменения.
    ///
    /// `None` в `settings.min_level` возвращает сервер к фильтру запуска.
    async fn save_log_settings(
        &self,
        settings: HostLogSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostLogSettings>;
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
    async fn insert_voice_activity_sample(&self, sample: VoiceActivitySample)
    -> anyhow::Result<()>;
    /// Возвращает снимки активности в полуинтервале `(since, now]` в хронологическом порядке.
    ///
    /// Границы времени выражены в UTC. Пустой `since` означает запрос всей доступной
    /// истории, но выборка дополнительно ограничивается 24 часами вызывающего кода.
    async fn load_voice_activity_samples(
        &self,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<VoiceActivitySample>>;
    /// Удаляет измерения старше указанного момента и возвращает число удалённых строк.
    async fn delete_voice_activity_samples_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> anyhow::Result<usize>;
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

    async fn load_host_owners(&self) -> anyhow::Result<Vec<HostOwner>> {
        use entities::host_owners;
        Ok(host_owners::Entity::find()
            .order_by_asc(host_owners::Column::GrantedAt)
            .order_by_asc(host_owners::Column::UserId)
            .all(&self.database)
            .await?
            .into_iter()
            .map(|model| HostOwner {
                user_id: model.user_id,
                granted_at: model.granted_at,
                granted_by_user_id: model.granted_by_user_id,
            })
            .collect())
    }

    async fn grant_host_owner(&self, owner: HostOwner) -> anyhow::Result<()> {
        use entities::host_owners;
        use sea_orm::sea_query::OnConflict;

        host_owners::Entity::insert(host_owners::ActiveModel {
            user_id: Set(owner.user_id),
            granted_at: Set(owner.granted_at),
            granted_by_user_id: Set(owner.granted_by_user_id),
        })
        .on_conflict(
            OnConflict::column(host_owners::Column::UserId)
                .update_columns([
                    host_owners::Column::GrantedAt,
                    host_owners::Column::GrantedByUserId,
                ])
                .to_owned(),
        )
        .exec(&self.database)
        .await?;
        Ok(())
    }

    async fn revoke_host_owner(&self, user_id: Uuid) -> anyhow::Result<RevokeHostOwnerOutcome> {
        use entities::host_owners;

        let transaction = self.database.begin().await?;
        // Одинаковый порядок блокировок предотвращает deadlock при взаимном
        // отзыве. В READ COMMITTED строки, удалённые ожидавшей транзакцией,
        // пропускаются после получения блокировки и не входят в число владельцев.
        let owners = host_owners::Entity::find()
            .order_by_asc(host_owners::Column::UserId)
            .lock(LockType::Update)
            .all(&transaction)
            .await?;
        let outcome = if !owners.iter().any(|owner| owner.user_id == user_id) {
            RevokeHostOwnerOutcome::Missing
        } else if owners.len() == 1 {
            RevokeHostOwnerOutcome::LastOwner
        } else {
            host_owners::Entity::delete_by_id(user_id)
                .exec(&transaction)
                .await?;
            RevokeHostOwnerOutcome::Revoked
        };
        transaction.commit().await?;
        Ok(outcome)
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

    async fn load_log_settings(&self) -> anyhow::Result<HostLogSettings> {
        use entities::host_log_settings;
        let Some(model) = host_log_settings::Entity::find_by_id(super::domain::LOG_SETTINGS_ID)
            .one(&self.database)
            .await?
        else {
            return Ok(HostLogSettings::default());
        };
        Ok(HostLogSettings {
            min_level: model
                .min_level
                .as_deref()
                .map(super::domain::LogLevel::parse)
                .transpose()?,
            updated_at: Some(model.updated_at),
        })
    }

    async fn save_log_settings(
        &self,
        settings: HostLogSettings,
        updated_by: Uuid,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<HostLogSettings> {
        use entities::host_log_settings;
        let active = host_log_settings::ActiveModel {
            id: Set(super::domain::LOG_SETTINGS_ID),
            min_level: Set(settings.min_level.map(|level| level.as_str().to_owned())),
            updated_at: Set(updated_at),
            updated_by_user_id: Set(Some(updated_by)),
        };
        if host_log_settings::Entity::find_by_id(super::domain::LOG_SETTINGS_ID)
            .one(&self.database)
            .await?
            .is_some()
        {
            active.update(&self.database).await?;
        } else {
            active.insert(&self.database).await?;
        }
        Ok(HostLogSettings {
            min_level: settings.min_level,
            updated_at: Some(updated_at),
        })
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

    async fn insert_voice_activity_sample(
        &self,
        sample: VoiceActivitySample,
    ) -> anyhow::Result<()> {
        use entities::host_voice_activity_samples;
        host_voice_activity_samples::ActiveModel {
            id: Set(sample.id),
            sampled_at: Set(sample.sampled_at),
            voice_connections: Set(i32::try_from(sample.voice_connections)?),
            video_sources: Set(i32::try_from(sample.video_sources)?),
        }
        .insert(&self.database)
        .await?;
        Ok(())
    }

    async fn load_voice_activity_samples(
        &self,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<VoiceActivitySample>> {
        use entities::host_voice_activity_samples;
        use sea_orm::QueryOrder;

        let models = host_voice_activity_samples::Entity::find()
            .filter(host_voice_activity_samples::Column::SampledAt.gt(since))
            .filter(host_voice_activity_samples::Column::SampledAt.lte(now))
            .order_by_asc(host_voice_activity_samples::Column::SampledAt)
            .all(&self.database)
            .await?;

        models
            .into_iter()
            .map(|model| {
                Ok(VoiceActivitySample {
                    id: model.id,
                    sampled_at: model.sampled_at,
                    voice_connections: u32::try_from(model.voice_connections)?,
                    video_sources: u32::try_from(model.video_sources)?,
                })
            })
            .collect()
    }

    async fn delete_voice_activity_samples_before(
        &self,
        cutoff: DateTime<Utc>,
    ) -> anyhow::Result<usize> {
        use entities::host_voice_activity_samples;
        let result = host_voice_activity_samples::Entity::delete_many()
            .filter(host_voice_activity_samples::Column::SampledAt.lt(cutoff))
            .exec(&self.database)
            .await?;
        Ok(usize::try_from(result.rows_affected)?)
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

#[cfg(test)]
mod tests;
