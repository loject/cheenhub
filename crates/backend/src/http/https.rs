//! Необязательный HTTPS TCP-слушатель для того же Axum-роутера.
//!
//! Не управляет сертификатами WebTransport и не заменяет HTTP-слушатель.

use std::{net::SocketAddr, time::Duration};

use anyhow::Context;
use axum::{Router, http::HeaderValue, middleware, response::Response};
use axum_server::{Handle, tls_rustls::RustlsConfig};

/// Слушатель HTTPS с управляемым завершением работы.
pub(crate) struct HttpsListener {
    handle: Handle<SocketAddr>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

/// Запускает HTTPS на выделенном TCP-порту с сертификатом WebTransport.
///
/// Ошибки загрузки сертификатов и привязки порта возвращаются до запуска.
/// Ошибка уже работающего TLS-сервера переводит backend в фазу остановки.
pub(crate) async fn start(
    address: SocketAddr,
    cert_path: &str,
    key_path: &str,
    app: Router,
) -> anyhow::Result<HttpsListener> {
    // Предпочитаем ring; уже установленный TLS-провайдер не заменяем.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let tls = RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .context("failed to load HTTPS TLS certificate and key")?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind HTTPS listener at {address}"))?;
    let bound_address = listener.local_addr()?;
    let listener = listener.into_std()?;
    let handle = Handle::new();
    let https_app = app.layer(middleware::map_response(
        |mut response: Response| async move {
            response.headers_mut().insert(
                "strict-transport-security",
                HeaderValue::from_static("max-age=63072000; includeSubDomains"),
            );
            response
        },
    ));
    let server = axum_server::from_tcp_rustls(listener, tls)
        .context("failed to initialize HTTPS listener")?
        .handle(handle.clone());
    let task = tokio::spawn(async move {
        let result = server
            .serve(https_app.into_make_service())
            .await
            .context("HTTPS server stopped with an error");
        if let Err(error) = &result {
            tracing::error!(%error, "HTTPS listener failed; draining backend");
            crate::lifecycle::lifecycle().begin_draining();
        }
        result
    });
    tracing::info!(address = %bound_address, "additional HTTPS listener started");
    Ok(HttpsListener { handle, task })
}

impl HttpsListener {
    /// Прекращает новые HTTPS-подключения и ожидает завершения активных запросов.
    pub(crate) async fn shutdown(self) -> anyhow::Result<()> {
        self.handle.graceful_shutdown(Some(Duration::from_secs(25)));
        self.task.await.context("HTTPS listener task failed")?
    }
}

#[cfg(test)]
mod tests;
