//! Backend logging and tracing setup.

use anyhow::anyhow;
use tracing_subscriber::{EnvFilter, fmt};

/// Initializes process-wide tracing.
pub(crate) fn init(filter: &str) -> anyhow::Result<()> {
    fmt()
        .with_env_filter(EnvFilter::new(filter))
        .try_init()
        .map_err(|error| anyhow!("failed to initialize tracing subscriber: {error}"))
}
