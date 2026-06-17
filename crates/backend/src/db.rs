//! Настройка подключения к базе данных.

use anyhow::Context;
use sea_orm::{Database, DatabaseConnection};
use tracing::info;

/// Открывает подключение к базе данных Postgres.
pub(crate) async fn connect(database_url: &str) -> anyhow::Result<DatabaseConnection> {
    let database = Database::connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    info!("connected to Postgres");
    Ok(database)
}
