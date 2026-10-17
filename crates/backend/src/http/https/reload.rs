//! Фоновая ротация TLS-сертификата HTTPS без перезапуска TCP-слушателя.
//!
//! Кандидат применяется только после двух последовательных одинаковых чтений
//! сертификата и ключа. Невалидная пара не заменяет действующую конфигурацию.

use std::time::Duration;

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use tokio::time::{MissedTickBehavior, interval};

const RELOAD_INTERVAL: Duration = Duration::from_secs(5);

/// Читает PEM-пару с диска, включая пути через symlink Certbot.
pub(super) fn read_pair(cert_path: &str, key_path: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let cert = std::fs::read(cert_path)
        .with_context(|| format!("failed to read HTTPS certificate at {cert_path}"))?;
    let key = std::fs::read(key_path)
        .with_context(|| format!("failed to read HTTPS private key at {key_path}"))?;
    Ok((cert, key))
}

struct ReloadState {
    applied: (Vec<u8>, Vec<u8>),
    pending: Option<(Vec<u8>, Vec<u8>)>,
    last_error: Option<String>,
}

impl ReloadState {
    fn new(cert: Vec<u8>, key: Vec<u8>) -> Self {
        Self {
            applied: (cert, key),
            pending: None,
            last_error: None,
        }
    }

    fn reject(&mut self, error: &str, cert_path: &str, key_path: &str) {
        self.pending = None;
        if self.last_error.as_deref() != Some(error) {
            tracing::warn!(
                cert_path,
                key_path,
                error,
                "HTTPS TLS candidate rejected; keeping active certificate"
            );
            self.last_error = Some(error.to_owned());
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PollOutcome {
    Unchanged,
    Detected,
    Applied,
    Rejected,
}

async fn poll_once(
    config: &RustlsConfig,
    cert_path: &str,
    key_path: &str,
    state: &mut ReloadState,
) -> PollOutcome {
    let candidate = match read_pair(cert_path, key_path) {
        Ok(candidate) => candidate,
        Err(error) => {
            state.reject(&error.to_string(), cert_path, key_path);
            return PollOutcome::Rejected;
        }
    };
    if candidate == state.applied {
        state.pending = None;
        state.last_error = None;
        return PollOutcome::Unchanged;
    }
    if state.pending.as_ref() != Some(&candidate) {
        state.pending = Some(candidate);
        tracing::debug!(
            cert_path,
            "HTTPS TLS candidate changed; waiting for stable PEM pair"
        );
        return PollOutcome::Detected;
    }
    state.pending = None;
    match config
        .reload_from_pem(candidate.0.clone(), candidate.1.clone())
        .await
    {
        Ok(()) => {
            state.applied = candidate;
            state.last_error = None;
            tracing::info!(
                cert_path,
                "HTTPS TLS certificate reloaded for new connections"
            );
            PollOutcome::Applied
        }
        Err(error) => {
            state.reject(&error.to_string(), cert_path, key_path);
            PollOutcome::Rejected
        }
    }
}

/// Запускает опрос PEM-файлов каждые пять секунд.
///
/// Задача завершается отменой её JoinHandle при остановке HTTPS-слушателя.
pub(super) fn spawn(
    config: RustlsConfig,
    cert_path: String,
    key_path: String,
    initial_cert: Vec<u8>,
    initial_key: Vec<u8>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut state = ReloadState::new(initial_cert, initial_key);
        let mut ticker = interval(RELOAD_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        ticker.tick().await;
        tracing::info!(
            cert_path,
            interval_seconds = 5,
            "HTTPS TLS reload watcher started"
        );
        loop {
            ticker.tick().await;
            let _ = poll_once(&config, &cert_path, &key_path, &mut state).await;
        }
    })
}

#[cfg(test)]
mod tests;
