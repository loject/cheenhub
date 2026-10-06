//! Конфигурация realtime-клиента.

use url::Url;

use super::error::RealtimeError;

/// Возвращает настроенный URL realtime-эндпойнта.
pub(crate) fn realtime_url() -> Result<Url, RealtimeError> {
    crate::config::realtime_webtransport_url()
        .map_err(|error| RealtimeError::new(format!("Invalid realtime URL: {error}")))
}

/// Возвращает настроенный URL realtime-эндпойнта для fallback через WebSocket.
pub(crate) fn realtime_websocket_url() -> Result<Url, RealtimeError> {
    crate::config::realtime_websocket_url()
        .map_err(|error| RealtimeError::new(format!("Invalid realtime WebSocket URL: {error}")))
}

/// Возвращает настроенный SHA-256 fingerprint realtime-сертификата.
pub(super) fn realtime_cert_sha256() -> Result<Option<Vec<u8>>, RealtimeError> {
    let Some(value) = option_env!("CHEENHUB_REALTIME_CERT_SHA256") else {
        return Ok(None);
    };
    parse_realtime_cert_sha256(value)
}

fn parse_realtime_cert_sha256(value: &str) -> Result<Option<Vec<u8>>, RealtimeError> {
    let normalized: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != ':')
        .collect();
    if normalized.is_empty() {
        return if value.is_empty() {
            Ok(None)
        } else {
            Err(RealtimeError::new(
                "CHEENHUB_REALTIME_CERT_SHA256 должен содержать ровно 64 шестнадцатеричных символа",
            ))
        };
    }

    if normalized.len() != 64 {
        return Err(RealtimeError::new(
            "CHEENHUB_REALTIME_CERT_SHA256 должен содержать ровно 64 шестнадцатеричных символа",
        ));
    }

    let mut bytes = Vec::with_capacity(32);
    for chunk in normalized.as_bytes().chunks(2) {
        if chunk.len() != 2 {
            return Err(RealtimeError::new(
                "CHEENHUB_REALTIME_CERT_SHA256 должен быть SHA-256 fingerprint в hex-формате",
            ));
        }
        let hex = std::str::from_utf8(chunk).map_err(|_| {
            RealtimeError::new("CHEENHUB_REALTIME_CERT_SHA256 должен быть корректным hex")
        })?;
        let byte = u8::from_str_radix(hex, 16).map_err(|_| {
            RealtimeError::new("CHEENHUB_REALTIME_CERT_SHA256 должен быть корректным hex")
        })?;
        bytes.push(byte);
    }

    Ok(Some(bytes))
}

/// Возвращает нормализованный SHA-256 fingerprint для browser WebTransport API.
#[allow(dead_code)]
pub(crate) fn realtime_cert_sha256_hex() -> Result<Option<String>, RealtimeError> {
    realtime_cert_sha256().map(|hash| {
        hash.map(|bytes| {
            bytes
                .into_iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        })
    })
}

#[cfg(test)]
mod tests;
