//! Настройка логирования и трассировки бэкенда.

use anyhow::anyhow;
use tracing_subscriber::{EnvFilter, fmt};

/// Инициализирует трассировку на уровне процесса.
pub(crate) fn init(filter: &str) -> anyhow::Result<()> {
    fmt()
        .with_env_filter(EnvFilter::new(filter))
        .try_init()
        .map_err(|error| anyhow!("failed to initialize tracing subscriber: {error}"))
}
