//! Проверки безопасной замены TLS-конфигурации и сохранения старой при ошибках.

use std::{fs, sync::Arc};

use axum_server::tls_rustls::RustlsConfig;

use super::{PollOutcome, ReloadState, poll_once};

struct Fixture(std::path::PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cheenhub-https-reload-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn paths(&self) -> (String, String) {
        (
            self.0.join("cert.pem").to_str().unwrap().to_owned(),
            self.0.join("key.pem").to_str().unwrap().to_owned(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn new_pair() -> (Vec<u8>, Vec<u8>) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    (
        cert.cert.pem().into_bytes(),
        cert.signing_key.serialize_pem().into_bytes(),
    )
}

#[tokio::test]
async fn stable_certificate_rotation_replaces_tls_config_on_second_poll() {
    let fixture = Fixture::new();
    let (cert_path, key_path) = fixture.paths();
    let initial = new_pair();
    fs::write(&cert_path, &initial.0).unwrap();
    fs::write(&key_path, &initial.1).unwrap();
    let config = RustlsConfig::from_pem(initial.0.clone(), initial.1.clone())
        .await
        .unwrap();
    let previous = config.get_inner();
    let mut state = ReloadState::new(initial.0, initial.1);
    let rotated = new_pair();
    fs::write(&cert_path, &rotated.0).unwrap();
    fs::write(&key_path, &rotated.1).unwrap();

    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Detected
    );
    assert!(Arc::ptr_eq(&previous, &config.get_inner()));
    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Applied
    );
    let current = config.get_inner();
    assert!(!Arc::ptr_eq(&previous, &current));
    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Unchanged
    );
    assert!(Arc::ptr_eq(&current, &config.get_inner()));
}

#[tokio::test]
async fn invalid_key_retains_active_certificate_until_valid_pair_arrives() {
    let fixture = Fixture::new();
    let (cert_path, key_path) = fixture.paths();
    let initial = new_pair();
    fs::write(&cert_path, &initial.0).unwrap();
    fs::write(&key_path, &initial.1).unwrap();
    let config = RustlsConfig::from_pem(initial.0.clone(), initial.1.clone())
        .await
        .unwrap();
    let current = config.get_inner();
    let mut state = ReloadState::new(initial.0, initial.1);
    let rotated = new_pair();
    let unrelated = new_pair();
    fs::write(&cert_path, &rotated.0).unwrap();
    fs::write(&key_path, &unrelated.1).unwrap();

    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Detected
    );
    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Rejected
    );
    assert!(Arc::ptr_eq(&current, &config.get_inner()));

    fs::write(&key_path, &rotated.1).unwrap();
    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Detected
    );
    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Applied
    );
    assert!(!Arc::ptr_eq(&current, &config.get_inner()));
}

#[tokio::test]
async fn unreadable_cert_keeps_active_configuration() {
    let fixture = Fixture::new();
    let (cert_path, key_path) = fixture.paths();
    let initial = new_pair();
    fs::write(&cert_path, &initial.0).unwrap();
    fs::write(&key_path, &initial.1).unwrap();
    let config = RustlsConfig::from_pem(initial.0.clone(), initial.1.clone())
        .await
        .unwrap();
    let active = config.get_inner();
    let mut state = ReloadState::new(initial.0, initial.1);
    fs::remove_file(&cert_path).unwrap();

    assert_eq!(
        poll_once(&config, &cert_path, &key_path, &mut state).await,
        PollOutcome::Rejected
    );
    assert!(Arc::ptr_eq(&active, &config.get_inner()));
}
