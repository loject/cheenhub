//! Android-реализация WebTransport с проверкой TLS через системный Trust Manager.

use std::sync::Arc;

use dioxus::prelude::{debug, info};
use jni::JavaVM;
use jni::objects::JObject;
use ndk_context::android_context;
use rustls_platform_verifier::BuilderVerifierExt;
use url::Url;
use web_transport::{ClientBuilder, Session};

use crate::features::realtime::config;
use crate::features::realtime::error::RealtimeError;

#[manganis::ffi("../../target/android-dependencies/rustls-platform-verifier.aar")]
unsafe extern "Kotlin" {}

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

    initialize_platform_verifier()?;
    let client = platform_client()?;
    info!("using Android Trust Manager for WebTransport realtime TLS");
    client
        .connect(url)
        .await
        .map(Into::into)
        .map_err(|error| RealtimeError::new(format!("Failed to connect realtime session: {error}")))
}

fn initialize_platform_verifier() -> Result<(), RealtimeError> {
    let context = android_context();
    // `ndk_context` возвращает VM текущего процесса, которая остаётся валидной
    // на всём протяжении жизни Android-приложения.
    let java_vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.map_err(|error| {
        RealtimeError::new(format!(
            "Failed to access Android JVM for realtime TLS: {error}"
        ))
    })?;
    let mut env = java_vm.attach_current_thread().map_err(|error| {
        RealtimeError::new(format!(
            "Failed to attach realtime TLS to Android JVM: {error}"
        ))
    })?;
    // `ndk_context` возвращает глобальный Context текущего приложения;
    // verifier сразу преобразует его в собственную global reference.
    let context = unsafe { JObject::from_raw(context.context().cast()) };
    rustls_platform_verifier::android::init_with_env(&mut env, context).map_err(|error| {
        RealtimeError::new(format!(
            "Failed to initialize Android Trust Manager for realtime TLS: {error}"
        ))
    })?;
    debug!("Android Trust Manager initialized for realtime TLS");
    Ok(())
}

fn platform_client() -> Result<web_transport::quinn::Client, RealtimeError> {
    let provider = web_transport::quinn::crypto::default_provider();
    let mut tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|error| {
            RealtimeError::new(format!(
                "Failed to configure TLS 1.3 for Android realtime: {error}"
            ))
        })?
        .with_platform_verifier()
        .map_err(|error| {
            RealtimeError::new(format!(
                "Failed to configure Android realtime certificate verifier: {error}"
            ))
        })?
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

    Ok(web_transport::quinn::Client::new(endpoint, quic_config))
}
