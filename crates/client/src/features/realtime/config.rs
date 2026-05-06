//! Realtime client configuration.

use url::Url;
use web_transport::ClientBuilder;

use super::error::RealtimeError;

const DEFAULT_REALTIME_URL: &str = "https://127.0.0.1:4443/realtime";

/// Returns the configured realtime endpoint URL.
pub(crate) fn realtime_url() -> Result<Url, RealtimeError> {
    Url::parse(option_env!("CHEENHUB_REALTIME_URL").unwrap_or(DEFAULT_REALTIME_URL))
        .map_err(|error| RealtimeError::new(format!("Invalid realtime URL: {error}")))
}

/// Builds a WebTransport client using either system roots or a configured cert hash.
pub(crate) fn realtime_client() -> Result<web_transport::Client, RealtimeError> {
    let builder = ClientBuilder::new();
    if let Some(hash) = realtime_cert_sha256()? {
        return builder
            .with_server_certificate_hashes(vec![hash])
            .map_err(|error| {
                RealtimeError::new(format!("Failed to create realtime client: {error}"))
            });
    }

    builder
        .with_system_roots()
        .map_err(|error| RealtimeError::new(format!("Failed to create realtime client: {error}")))
}

fn realtime_cert_sha256() -> Result<Option<Vec<u8>>, RealtimeError> {
    let Some(value) = option_env!("CHEENHUB_REALTIME_CERT_SHA256") else {
        return Ok(None);
    };
    let normalized: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != ':')
        .collect();
    if normalized.is_empty() {
        return Ok(None);
    }

    let mut bytes = Vec::with_capacity(normalized.len() / 2);
    for chunk in normalized.as_bytes().chunks(2) {
        if chunk.len() != 2 {
            return Err(RealtimeError::new(
                "CHEENHUB_REALTIME_CERT_SHA256 must be a hex SHA-256 fingerprint",
            ));
        }
        let hex = std::str::from_utf8(chunk)
            .map_err(|_| RealtimeError::new("CHEENHUB_REALTIME_CERT_SHA256 must be valid hex"))?;
        let byte = u8::from_str_radix(hex, 16)
            .map_err(|_| RealtimeError::new("CHEENHUB_REALTIME_CERT_SHA256 must be valid hex"))?;
        bytes.push(byte);
    }

    Ok(Some(bytes))
}
