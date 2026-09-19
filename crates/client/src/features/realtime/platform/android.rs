//! Android-реализация WebTransport с проверкой TLS через встроенные публичные WebPKI roots.

use std::sync::Arc;

use dioxus::prelude::{debug, info};
use url::Url;
use web_transport::{ClientBuilder, Session};

use crate::features::realtime::config;
use crate::features::realtime::error::RealtimeError;

pub(in crate::features::realtime) async fn connect(url: Url) -> Result<Session, RealtimeError> {
    if let Some(hash) = config::realtime_cert_sha256()? {
        debug!("using configured certificate fingerprint for Android WebTransport realtime");
        let client = ClientBuilder::new()
            .with_server_certificate_hashes(vec![hash])
            .map_err(|error| {
                RealtimeError::new(format!("Failed to create realtime client: {error}"))
            })?;
        return client.connect(url).await.map_err(|error| {
            RealtimeError::new(format!("Failed to connect realtime session: {error}"))
        });
    }

    let client = public_roots_client()?;
    client
        .connect(url)
        .await
        .map(Into::into)
        .map_err(|error| RealtimeError::new(format!("Failed to connect realtime session: {error}")))
}

fn public_roots_client() -> Result<web_transport::quinn::Client, RealtimeError> {
    let provider = web_transport::quinn::crypto::default_provider();
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let mut tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| {
            RealtimeError::new(format!(
                "Failed to configure TLS 1.3 for Android realtime: {error}"
            ))
        })?
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![web_transport::quinn::ALPN.as_bytes().to_vec()];

    let quic_crypto = web_transport::quinn::quinn::crypto::rustls::QuicClientConfig::try_from(tls)
        .map_err(|error| {
            RealtimeError::new(format!("Failed to configure Android QUIC TLS: {error}"))
        })?;
    let quic_config = web_transport::quinn::quinn::ClientConfig::new(Arc::new(quic_crypto));
    let endpoint =
        web_transport::quinn::quinn::Endpoint::client("[::]:0".parse().map_err(|error| {
            RealtimeError::new(format!(
                "Failed to parse Android QUIC bind address: {error}"
            ))
        })?)
        .map_err(|error| {
            RealtimeError::new(format!("Failed to create Android QUIC endpoint: {error}"))
        })?;

    info!("using bundled WebPKI roots for Android WebTransport realtime TLS");
    Ok(web_transport::quinn::Client::new(endpoint, quic_config))
}
