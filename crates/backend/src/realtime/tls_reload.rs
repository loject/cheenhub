//! Горячая замена TLS-конфигурации WebTransport.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use rustls_pki_types::CertificateDer;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

use super::tls::{
    TlsConfig, build_server_config_from_parts, certificate_chain_sha256_hex,
    certificate_sha256_hex, load_certificates, load_private_key,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CertificateIdentity {
    leaf_fingerprint: String,
    chain_fingerprint: String,
    key_source: KeySourceIdentity,
}

/// Метаданные источника ключа без чтения или хеширования его содержимого.
#[derive(Debug, Clone, PartialEq, Eq)]
enum KeySourceIdentity {
    /// Метаданные файла ключа в Unix, включая inode после смены Certbot symlink.
    #[cfg(unix)]
    Unix {
        canonical_path: PathBuf,
        device: u64,
        inode: u64,
        length: u64,
        modified: Option<SystemTime>,
    },
    /// Переносимый набор метаданных, когда inode недоступен.
    #[cfg(not(unix))]
    Portable {
        canonical_path: PathBuf,
        length: u64,
        modified: Option<SystemTime>,
    },
}

/// Запускает фоновую проверку TLS WebTransport без замены UDP-слушателя.
pub(crate) fn spawn_tls_reloader(
    endpoint: quinn::Endpoint,
    tls: TlsConfig,
    reload_interval_seconds: u64,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let result = watch_tls(endpoint, tls, reload_interval_seconds).await;
        if let Err(error) = &result {
            tracing::error!(%error, "WebTransport TLS reload watcher failed");
        }
        result
    })
}

async fn watch_tls(
    endpoint: quinn::Endpoint,
    tls: TlsConfig,
    reload_interval_seconds: u64,
) -> Result<()> {
    let mut pipeline = ReloadPipeline::new();
    let mut ticker = interval(Duration::from_secs(reload_interval_seconds));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker.tick().await;
    info!(cert_path = %tls.cert_path, key_path = %tls.key_path, reload_interval_seconds, "WebTransport TLS reload watcher started");

    loop {
        ticker.tick().await;
        let _ = poll_once(&endpoint, &tls.cert_path, &tls.key_path, &mut pipeline);
    }
}

/// Состояние примененной конфигурации и debounce одного TLS-наблюдателя.
pub(super) struct ReloadPipeline {
    applied: Option<CertificateIdentity>,
    state: ReloadState,
    last_invalid_error: Option<String>,
}

impl ReloadPipeline {
    /// Создает pipeline без предположения о файловой identity initial bind.
    pub(super) fn new() -> Self {
        Self {
            applied: None,
            state: ReloadState::default(),
            last_invalid_error: None,
        }
    }
}

/// Выполняет одну проверку файлов и при необходимости заменяет TLS config endpoint.
pub(super) fn poll_once(
    endpoint: &quinn::Endpoint,
    cert_path: &str,
    key_path: &str,
    pipeline: &mut ReloadPipeline,
) -> PollOutcome {
    let candidate = match load_tls_candidate(cert_path, key_path) {
        Ok(candidate) => candidate,
        Err(error) => {
            let error = error.to_string();
            if pipeline.last_invalid_error.as_deref() != Some(error.as_str()) {
                warn!(cert_path, key_path, %error, "WebTransport TLS reload candidate is invalid; retaining active configuration");
                pipeline.last_invalid_error = Some(error);
            }
            pipeline.state.clear_pending();
            return PollOutcome::Rejected;
        }
    };
    pipeline.last_invalid_error = None;
    match pipeline
        .state
        .observe(candidate.identity.clone(), pipeline.applied.as_ref())
    {
        ReloadDecision::Unchanged => PollOutcome::Unchanged,
        ReloadDecision::Detected => {
            info!(old_leaf_fingerprint = ?pipeline.applied.as_ref().map(|identity| &identity.leaf_fingerprint), new_leaf_fingerprint = %candidate.identity.leaf_fingerprint, new_chain_fingerprint = %candidate.identity.chain_fingerprint, "WebTransport TLS reload candidate detected");
            PollOutcome::Detected
        }
        ReloadDecision::Apply => {
            endpoint.set_server_config(Some(candidate.config));
            info!(old_leaf_fingerprint = ?pipeline.applied.as_ref().map(|identity| &identity.leaf_fingerprint), new_leaf_fingerprint = %candidate.identity.leaf_fingerprint, "WebTransport TLS configuration reloaded for new connections");
            pipeline.applied = Some(candidate.identity);
            PollOutcome::Applied
        }
    }
}

/// Результат одной проверки TLS-наблюдателя.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum PollOutcome {
    /// Файлы соответствуют примененной конфигурации.
    Unchanged,
    /// Новый валидный candidate ожидает второго одинакового наблюдения.
    Detected,
    /// Новый валидный candidate применен для новых QUIC соединений.
    Applied,
    /// Candidate невалиден, активная конфигурация сохранена.
    Rejected,
}

struct TlsCandidate {
    identity: CertificateIdentity,
    config: quinn::ServerConfig,
}

fn load_tls_candidate(cert_path: &str, key_path: &str) -> Result<TlsCandidate> {
    let certificates = load_certificates(cert_path)?;
    let identity = certificate_identity_from_chain(&certificates, key_source_identity(key_path)?);
    let private_key = load_private_key(key_path)?;
    let config = build_server_config_from_parts(certificates, private_key)?;
    Ok(TlsCandidate { identity, config })
}

fn certificate_identity_from_chain(
    certificates: &[CertificateDer<'_>],
    key_source: KeySourceIdentity,
) -> CertificateIdentity {
    CertificateIdentity {
        leaf_fingerprint: certificate_sha256_hex(&certificates[0]),
        chain_fingerprint: certificate_chain_sha256_hex(certificates),
        key_source,
    }
}

fn key_source_identity(key_path: &str) -> Result<KeySourceIdentity> {
    let canonical_path = std::fs::canonicalize(key_path)?;
    let metadata = std::fs::metadata(&canonical_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        Ok(KeySourceIdentity::Unix {
            canonical_path,
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }
    #[cfg(not(unix))]
    Ok(KeySourceIdentity::Portable {
        canonical_path,
        length: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[derive(Default)]
struct ReloadState {
    pending: Option<CertificateIdentity>,
}

enum ReloadDecision {
    Unchanged,
    Detected,
    Apply,
}

impl ReloadState {
    fn observe(
        &mut self,
        candidate: CertificateIdentity,
        applied: Option<&CertificateIdentity>,
    ) -> ReloadDecision {
        if applied == Some(&candidate) {
            self.clear_pending();
            return ReloadDecision::Unchanged;
        }
        if self.pending.as_ref() == Some(&candidate) {
            self.clear_pending();
            return ReloadDecision::Apply;
        }
        self.pending = Some(candidate);
        ReloadDecision::Detected
    }

    fn clear_pending(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_state_debounces_chain_changes_and_invalid_candidates() {
        let applied = CertificateIdentity {
            leaf_fingerprint: "leaf-a".to_owned(),
            chain_fingerprint: "chain-a".to_owned(),
            key_source: key_source("key-a"),
        };
        let changed_chain = CertificateIdentity {
            leaf_fingerprint: "leaf-a".to_owned(),
            chain_fingerprint: "chain-b".to_owned(),
            key_source: key_source("key-a"),
        };
        let changed_leaf = CertificateIdentity {
            leaf_fingerprint: "leaf-b".to_owned(),
            chain_fingerprint: "chain-c".to_owned(),
            key_source: key_source("key-b"),
        };
        let mut state = ReloadState::default();
        assert!(matches!(
            state.observe(applied.clone(), Some(&applied)),
            ReloadDecision::Unchanged
        ));
        assert!(matches!(
            state.observe(changed_chain.clone(), Some(&applied)),
            ReloadDecision::Detected
        ));
        assert!(matches!(
            state.observe(changed_chain, Some(&applied)),
            ReloadDecision::Apply
        ));
        // Ошибка чтения пары сбрасывает подтверждение, не заменяя active config.
        state.clear_pending();
        assert!(matches!(
            state.observe(changed_leaf.clone(), Some(&applied)),
            ReloadDecision::Detected
        ));
        assert!(matches!(
            state.observe(changed_leaf, Some(&applied)),
            ReloadDecision::Apply
        ));
    }

    fn key_source(name: &str) -> KeySourceIdentity {
        #[cfg(unix)]
        {
            KeySourceIdentity::Unix {
                canonical_path: PathBuf::from(name),
                device: 1,
                inode: 1,
                length: 1,
                modified: None,
            }
        }
        #[cfg(not(unix))]
        {
            KeySourceIdentity::Portable {
                canonical_path: PathBuf::from(name),
                length: 1,
                modified: None,
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn key_symlink_target_change_requires_two_polls() {
        use std::os::unix::fs::symlink;

        let directory =
            std::env::temp_dir().join(format!("cheenhub-key-source-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("test directory is created");
        let first = directory.join("first-key.pem");
        let second = directory.join("second-key.pem");
        let link = directory.join("current-key.pem");
        std::fs::write(&first, b"same length key").expect("first key is written");
        std::fs::write(&second, b"same length key").expect("second key is written");
        symlink(&first, &link).expect("first key symlink is created");
        let applied = CertificateIdentity {
            leaf_fingerprint: "leaf".to_owned(),
            chain_fingerprint: "chain".to_owned(),
            key_source: key_source_identity(link.to_str().expect("utf-8 path"))
                .expect("identity loads"),
        };
        std::fs::remove_file(&link).expect("old symlink is removed");
        symlink(&second, &link).expect("second key symlink is created");
        let changed = CertificateIdentity {
            leaf_fingerprint: "leaf".to_owned(),
            chain_fingerprint: "chain".to_owned(),
            key_source: key_source_identity(link.to_str().expect("utf-8 path"))
                .expect("identity loads"),
        };
        assert_ne!(applied.key_source, changed.key_source);
        let mut state = ReloadState::default();
        assert!(matches!(
            state.observe(changed.clone(), Some(&applied)),
            ReloadDecision::Detected
        ));
        assert!(matches!(
            state.observe(changed, Some(&applied)),
            ReloadDecision::Apply
        ));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn key_metadata_change_requires_two_polls() {
        let directory =
            std::env::temp_dir().join(format!("cheenhub-key-source-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("test directory is created");
        let key = directory.join("key.pem");
        std::fs::write(&key, b"key").expect("key is written");
        let applied_source =
            key_source_identity(key.to_str().expect("utf-8 path")).expect("identity loads");
        std::fs::write(&key, b"key with changed metadata").expect("key is replaced");
        let changed_source =
            key_source_identity(key.to_str().expect("utf-8 path")).expect("identity loads");
        assert_ne!(applied_source, changed_source);
        let applied = CertificateIdentity {
            leaf_fingerprint: "leaf".to_owned(),
            chain_fingerprint: "chain".to_owned(),
            key_source: applied_source,
        };
        let changed = CertificateIdentity {
            leaf_fingerprint: "leaf".to_owned(),
            chain_fingerprint: "chain".to_owned(),
            key_source: changed_source,
        };
        let mut state = ReloadState::default();
        assert!(matches!(
            state.observe(changed.clone(), Some(&applied)),
            ReloadDecision::Detected
        ));
        assert!(matches!(
            state.observe(changed, Some(&applied)),
            ReloadDecision::Apply
        ));
        let _ = std::fs::remove_dir_all(directory);
    }
}
