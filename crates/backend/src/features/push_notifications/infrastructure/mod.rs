//! Хранение push-установок и постоянной очереди доставки.

mod entities;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

use self::entities::{delivery_queue, installations};
use crate::features::push_notifications::domain::{
    DirectMessagePush, PendingDelivery, PushInstallation,
};

/// Конкретное Postgres-хранилище push-уведомлений.
#[derive(Clone)]
pub(crate) struct PostgresPushStore {
    database: DatabaseConnection,
}

impl PostgresPushStore {
    /// Создаёт хранилище поверх существующего подключения.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }

    /// Регистрирует установку в текущей auth-сессии.
    pub(crate) async fn upsert_installation(
        &self,
        id: Uuid,
        user_id: Uuid,
        session_id: Uuid,
        platform: &str,
        token: String,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let transaction = self.database.begin().await?;
        if let Some(existing) = installations::Entity::find_by_id(id)
            .one(&transaction)
            .await?
            && installation_owner_changed(
                existing.user_id,
                existing.session_id,
                user_id,
                session_id,
            )
        {
            delivery_queue::Entity::delete_many()
                .filter(delivery_queue::Column::InstallationId.eq(id))
                .exec(&transaction)
                .await?;
        }

        installations::Entity::delete_many()
            .filter(installations::Column::Token.eq(token.clone()))
            .filter(installations::Column::Id.ne(id))
            .exec(&transaction)
            .await?;

        let existing = installations::Entity::find_by_id(id)
            .one(&transaction)
            .await?;
        match existing {
            Some(existing) => {
                let mut active: installations::ActiveModel = existing.into();
                active.user_id = Set(user_id);
                active.session_id = Set(session_id);
                active.platform = Set(platform.to_owned());
                active.token = Set(token);
                active.active = Set(true);
                active.updated_at = Set(now);
                active.update(&transaction).await?;
            }
            None => {
                installations::ActiveModel {
                    id: Set(id),
                    user_id: Set(user_id),
                    session_id: Set(session_id),
                    platform: Set(platform.to_owned()),
                    token: Set(token),
                    active: Set(true),
                    created_at: Set(now),
                    updated_at: Set(now),
                }
                .insert(&transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Удаляет установку только из той же пользовательской auth-сессии.
    pub(crate) async fn delete_installation(
        &self,
        id: Uuid,
        user_id: Uuid,
        session_id: Uuid,
    ) -> anyhow::Result<bool> {
        let result = installations::Entity::delete_many()
            .filter(installations::Column::Id.eq(id))
            .filter(installations::Column::UserId.eq(user_id))
            .filter(installations::Column::SessionId.eq(session_id))
            .exec(&self.database)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Возвращает активные установки пользователя.
    pub(crate) async fn active_installations(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Vec<PushInstallation>> {
        Ok(installations::Entity::find()
            .filter(installations::Column::UserId.eq(user_id))
            .filter(installations::Column::Active.eq(true))
            .all(&self.database)
            .await?
            .into_iter()
            .map(|model| PushInstallation {
                id: model.id,
                session_id: model.session_id,
            })
            .collect())
    }

    /// Ставит отдельное идемпотентное задание для каждой установки.
    pub(crate) async fn enqueue(
        &self,
        installation_id: Uuid,
        message_id: Uuid,
        payload: &DirectMessagePush,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if delivery_queue::Entity::find()
            .filter(delivery_queue::Column::InstallationId.eq(installation_id))
            .filter(delivery_queue::Column::MessageId.eq(message_id))
            .one(&self.database)
            .await?
            .is_some()
        {
            return Ok(());
        }
        delivery_queue::ActiveModel {
            id: Set(Uuid::new_v4()),
            installation_id: Set(installation_id),
            message_id: Set(message_id),
            payload: Set(serde_json::to_value(payload)?),
            attempts: Set(0),
            next_attempt_at: Set(now),
            created_at: Set(now),
        }
        .insert(&self.database)
        .await
        .context("failed to enqueue push delivery")?;
        Ok(())
    }

    /// Загружает небольшую пачку готовых к отправке заданий.
    pub(crate) async fn due_deliveries(
        &self,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Vec<PendingDelivery>> {
        let jobs = delivery_queue::Entity::find()
            .filter(delivery_queue::Column::NextAttemptAt.lte(now))
            .order_by_asc(delivery_queue::Column::NextAttemptAt)
            .limit(32)
            .all(&self.database)
            .await?;
        let mut pending = Vec::new();
        for job in jobs {
            let Some(installation) = installations::Entity::find_by_id(job.installation_id)
                .filter(installations::Column::Active.eq(true))
                .one(&self.database)
                .await?
            else {
                delivery_queue::Entity::delete_by_id(job.id)
                    .exec(&self.database)
                    .await?;
                continue;
            };
            pending.push(PendingDelivery {
                id: job.id,
                installation_id: installation.id,
                session_id: installation.session_id,
                token: installation.token,
                attempts: job.attempts,
                payload: serde_json::from_value(job.payload)?,
            });
        }
        Ok(pending)
    }

    /// Удаляет задания старше допустимого срока системного уведомления.
    pub(crate) async fn prune_expired(&self, expires_before: DateTime<Utc>) -> anyhow::Result<u64> {
        Ok(delivery_queue::Entity::delete_many()
            .filter(delivery_queue::Column::CreatedAt.lt(expires_before))
            .exec(&self.database)
            .await?
            .rows_affected)
    }

    /// Удаляет успешно доставленное или окончательно отклонённое задание.
    pub(crate) async fn complete(&self, id: Uuid) -> anyhow::Result<()> {
        delivery_queue::Entity::delete_by_id(id)
            .exec(&self.database)
            .await?;
        Ok(())
    }

    /// Откладывает повторную попытку с ограниченным exponential backoff.
    pub(crate) async fn retry(&self, id: Uuid, attempts: i32) -> anyhow::Result<()> {
        let Some(model) = delivery_queue::Entity::find_by_id(id)
            .one(&self.database)
            .await?
        else {
            return Ok(());
        };
        let exponent = u32::try_from(attempts.clamp(1, 10)).unwrap_or(10);
        let delay_seconds = (5_i64.saturating_mul(2_i64.pow(exponent))).min(3600);
        let mut active: delivery_queue::ActiveModel = model.into();
        active.attempts = Set(attempts);
        active.next_attempt_at = Set(Utc::now() + Duration::seconds(delay_seconds));
        active.update(&self.database).await?;
        Ok(())
    }

    /// Деактивирует установку после окончательного ответа push-провайдера.
    pub(crate) async fn deactivate(&self, id: Uuid) -> anyhow::Result<()> {
        if let Some(model) = installations::Entity::find_by_id(id)
            .one(&self.database)
            .await?
        {
            let mut active: installations::ActiveModel = model.into();
            active.active = Set(false);
            active.updated_at = Set(Utc::now());
            active.update(&self.database).await?;
        }
        Ok(())
    }
}

fn installation_owner_changed(
    current_user_id: Uuid,
    current_session_id: Uuid,
    next_user_id: Uuid,
    next_session_id: Uuid,
) -> bool {
    current_user_id != next_user_id || current_session_id != next_session_id
}

#[cfg(test)]
mod tests {
    use super::installation_owner_changed;
    use uuid::Uuid;

    #[test]
    fn user_or_session_rebind_requires_old_delivery_cleanup() {
        let first_user = Uuid::new_v4();
        let second_user = Uuid::new_v4();
        let first_session = Uuid::new_v4();
        let second_session = Uuid::new_v4();

        assert!(installation_owner_changed(
            first_user,
            first_session,
            second_user,
            first_session
        ));
        assert!(installation_owner_changed(
            first_user,
            first_session,
            first_user,
            second_session
        ));
        assert!(!installation_owner_changed(
            first_user,
            first_session,
            first_user,
            first_session
        ));
    }
}
