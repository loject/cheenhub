//! Native-реализация WebTransport с системным хранилищем корневых сертификатов.

use url::Url;
use web_transport::{ClientBuilder, Session};

use crate::features::realtime::config;
use crate::features::realtime::error::RealtimeError;

pub(in crate::features::realtime) async fn connect(url: Url) -> Result<Session, RealtimeError> {
    let builder = ClientBuilder::new();
    let client = match config::realtime_cert_sha256()? {
        Some(hash) => builder.with_server_certificate_hashes(vec![hash]),
        None => builder.with_system_roots(),
    }
    .map_err(|error| RealtimeError::new(format!("Failed to create realtime client: {error}")))?;

    client
        .connect(url)
        .await
        .map_err(|error| RealtimeError::new(format!("Failed to connect realtime session: {error}")))
}
