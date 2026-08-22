//! Интеграционные проверки замены TLS-конфигурации QUIC.

use std::fs;
use std::sync::Arc;

use quinn::crypto::rustls::QuicClientConfig;
use rcgen::{CertificateParams, KeyPair};
use uuid::Uuid;

use super::tls::{build_server_config, load_certificates};
use super::tls_reload::{PollOutcome, ReloadPipeline, poll_once};

#[tokio::test]
async fn watcher_pipeline_rejects_invalid_pair_then_debounces_and_swaps_tls() {
    let directory = std::env::temp_dir().join(format!("cheenhub-quinn-test-{}", Uuid::new_v4()));
    let (first_cert, first_key) = generate_tls_pair(&directory, "first");
    let (second_cert, second_key) = generate_tls_pair(&directory, "second");
    let active_cert = directory.join("active-cert.pem");
    let active_key = directory.join("active-key.pem");
    fs::copy(&first_cert, &active_cert).expect("initial certificate is copied");
    fs::copy(&first_key, &active_key).expect("initial key is copied");
    let active_cert = active_cert.to_str().expect("utf-8 cert path").to_owned();
    let active_key = active_key.to_str().expect("utf-8 key path").to_owned();
    let server = quinn::Endpoint::server(
        build_server_config(&active_cert, &active_key).expect("initial server config"),
        "127.0.0.1:0".parse().expect("loopback address"),
    )
    .expect("server binds");
    let address = server.local_addr().expect("server address");
    let mut pipeline = ReloadPipeline::new();

    let first_server = tokio::spawn(accept_connection(server.clone()));
    let first_client = client_endpoint(&first_cert)
        .connect(address, "localhost")
        .expect("connection starts")
        .await
        .expect("first certificate is trusted");
    let first_server = first_server.await.expect("server task completes");

    assert_eq!(
        poll_once(&server, &active_cert, &active_key, &mut pipeline),
        PollOutcome::Detected
    );
    assert_eq!(
        poll_once(&server, &active_cert, &active_key, &mut pipeline),
        PollOutcome::Applied
    );

    fs::copy(&second_cert, &active_cert).expect("mismatched certificate is copied");
    assert_eq!(
        poll_once(&server, &active_cert, &active_key, &mut pipeline),
        PollOutcome::Rejected
    );
    let old_server = tokio::spawn(accept_connection(server.clone()));
    let old_client = client_endpoint(&first_cert)
        .connect(address, "localhost")
        .expect("connection starts")
        .await
        .expect("old certificate remains active after rejected pair");
    let _old_server = old_server.await.expect("server task completes");
    old_client.close(0_u32.into(), b"test complete");

    fs::copy(&second_key, &active_key).expect("matching key is copied");
    assert_eq!(
        poll_once(&server, &active_cert, &active_key, &mut pipeline),
        PollOutcome::Detected
    );
    assert_eq!(
        poll_once(&server, &active_cert, &active_key, &mut pipeline),
        PollOutcome::Applied
    );
    let second_server = tokio::spawn(accept_connection(server.clone()));
    let _second_client = client_endpoint(&second_cert)
        .connect(address, "localhost")
        .expect("connection starts")
        .await
        .expect("new certificate is trusted after second poll");
    let _second_server = second_server.await.expect("server task completes");
    let stale_server = tokio::spawn(accept_attempt(server.clone()));
    let stale_client = client_endpoint(&first_cert)
        .connect(address, "localhost")
        .expect("connection starts")
        .await;
    assert!(
        stale_client.is_err(),
        "new connection must reject the old certificate trust root"
    );
    let _ = stale_server.await.expect("server task completes");

    let mut send = first_client
        .open_uni()
        .await
        .expect("existing connection is open");
    send.write_all(b"still-open")
        .await
        .expect("existing connection writes after swap");
    send.finish().expect("stream finishes");
    let mut receive = first_server
        .accept_uni()
        .await
        .expect("server receives stream");
    assert_eq!(
        receive.read_to_end(64).await.expect("server reads stream"),
        b"still-open"
    );
    assert!(first_client.close_reason().is_none());

    server.close(0_u32.into(), b"test complete");
    let _ = fs::remove_dir_all(directory);
}

fn generate_tls_pair(directory: &std::path::Path, name: &str) -> (String, String) {
    fs::create_dir_all(directory).expect("test directory is created");
    let key = KeyPair::generate().expect("key is generated");
    let params = CertificateParams::new(vec!["localhost".to_owned()]).expect("params build");
    let certificate = params.self_signed(&key).expect("certificate is signed");
    let cert_path = directory.join(format!("{name}-cert.pem"));
    let key_path = directory.join(format!("{name}-key.pem"));
    fs::write(&cert_path, certificate.pem()).expect("certificate is written");
    fs::write(&key_path, key.serialize_pem()).expect("key is written");
    (
        cert_path.to_str().expect("utf-8 cert path").to_owned(),
        key_path.to_str().expect("utf-8 key path").to_owned(),
    )
}

fn client_endpoint(cert_path: &str) -> quinn::Endpoint {
    let certificates = load_certificates(cert_path).expect("certificate loads");
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(certificates[0].clone())
        .expect("root certificate adds");
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .expect("TLS 1.3 config")
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![web_transport_quinn::ALPN.as_bytes().to_vec()];
    let config = QuicClientConfig::try_from(config).expect("QUIC client config");
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().expect("loopback address"))
        .expect("client binds");
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(config)));
    endpoint
}

async fn accept_connection(endpoint: quinn::Endpoint) -> quinn::Connection {
    endpoint
        .accept()
        .await
        .expect("incoming connection")
        .await
        .expect("connection accepts")
}

async fn accept_attempt(
    endpoint: quinn::Endpoint,
) -> Result<quinn::Connection, quinn::ConnectionError> {
    endpoint.accept().await.expect("incoming connection").await
}
