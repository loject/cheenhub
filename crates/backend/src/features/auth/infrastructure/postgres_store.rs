//! Подключение и обслуживание постоянного хранилища аутентификации.

use chrono::{DateTime, Utc};
use sea_orm::DatabaseConnection;

/// Постоянное хранилище аутентификации в PostgreSQL.
pub(crate) struct PostgresAuthStore {
    /// Общее подключение для адаптеров инфраструктуры аутентификации.
    pub(super) database: DatabaseConnection,
}

impl PostgresAuthStore {
    /// Создаёт хранилище с общим подключением PostgreSQL.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }

    /// Удаляет порцию истёкших desktop-попыток вместе с их временными OAuth-записями.
    pub(crate) async fn cleanup_desktop_oauth(&self, now: DateTime<Utc>) -> anyhow::Result<u64> {
        super::postgres_desktop_oauth_cleanup::cleanup(&self.database, now).await
    }
}
