//! Database connection setup.

use anyhow::Context;
use sea_orm::{Database, DatabaseConnection};
use tracing::info;

/// Opens a Postgres database connection.
pub(crate) async fn connect(database_url: &str) -> anyhow::Result<DatabaseConnection> {
    let database = Database::connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    info!("connected to Postgres");
    Ok(database)
}
