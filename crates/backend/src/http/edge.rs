//! Необязательный внешний HTTP-слушатель: ACME HTTP-01 и редирект на HTTPS.
//!
//! Внутренний HTTP backend не перенаправляется, чтобы не ломать Nginx.

use std::{net::SocketAddr, path::Path};

use anyhow::{Context, anyhow};
use axum::{
    Router,
    http::{HeaderValue, StatusCode, Uri, header},
    middleware,
    response::{IntoResponse, Response},
};
use tower_http::services::ServeDir;
use url::Url;

/// Перенаправляет все запросы HTTP на публичный HTTPS origin нативного сервера.
///
/// Код 308 сохраняет метод и тело запроса; Host клиента не влияет на адрес назначения.
pub(crate) fn redirect_router(https_api_url: &str) -> anyhow::Result<Router> {
    let url = Url::parse(https_api_url).context("invalid CHEENHUB_BASE_URL for HTTP redirect")?;
    if url.scheme() != "https" {
        return Err(anyhow!("direct HTTPS requires HTTPS CHEENHUB_BASE_URL"));
    }
    let origin = url.origin().ascii_serialization();
    Ok(Router::new().fallback(move |uri: Uri| {
        let target = format!(
            "{origin}{}",
            uri.path_and_query().map_or("/", |part| part.as_str())
        );
        async move { axum::response::Redirect::permanent(&target) }
    }))
}

/// Запускает отдельный HTTP-слушатель для проверки Certbot и редиректов.
///
/// Принимает HTTPS API URL, выделяет из него канонический origin и отдаёт
/// файлы challenge из `webroot/.well-known/acme-challenge`.
pub(crate) async fn start(
    address: SocketAddr,
    https_api_url: &str,
    webroot: &Path,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
    let app = router(https_api_url, webroot)?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind edge HTTP listener at {address}"))?;
    let bound = listener.local_addr()?;
    tracing::info!(address = %bound, webroot = %webroot.display(), "edge HTTP listener started");
    Ok(tokio::spawn(async move {
        let lifecycle = crate::lifecycle::lifecycle();
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(lifecycle.wait_for_draining())
            .await
            .context("edge HTTP server stopped with an error");
        if let Err(error) = &result {
            tracing::error!(%error, "edge HTTP listener failed; draining backend");
            lifecycle.begin_draining();
        }
        result
    }))
}

fn router(https_api_url: &str, webroot: &Path) -> anyhow::Result<Router> {
    let url = Url::parse(https_api_url).context("invalid CHEENHUB_BASE_URL for edge HTTP")?;
    if url.scheme() != "https" {
        return Err(anyhow!(
            "BACKEND_EDGE_HTTP_PORT requires HTTPS CHEENHUB_BASE_URL"
        ));
    }
    if !webroot.is_dir() {
        return Err(anyhow!(
            "BACKEND_ACME_WEBROOT must be an existing directory"
        ));
    }
    let origin = url.origin().ascii_serialization();
    let challenge = Router::new()
        .fallback_service(ServeDir::new(webroot.join(".well-known/acme-challenge")))
        .layer(middleware::map_response(
            |mut response: Response| async move {
                if response.status().is_success() {
                    response.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("text/plain; charset=utf-8"),
                    );
                    response
                        .headers_mut()
                        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                }
                response
            },
        ));
    Ok(Router::new()
        .nest("/.well-known/acme-challenge", challenge)
        .fallback(move |uri: Uri| {
            let origin = origin.clone();
            async move {
                let path = uri.path_and_query().map_or("/", |part| part.as_str());
                match HeaderValue::from_str(&format!("{origin}{path}")) {
                    Ok(location) => (
                        StatusCode::MOVED_PERMANENTLY,
                        [(header::LOCATION, location)],
                    )
                        .into_response(),
                    Err(error) => {
                        tracing::warn!(%error, "invalid edge HTTP redirect target");
                        StatusCode::BAD_REQUEST.into_response()
                    }
                }
            }
        }))
}

#[cfg(test)]
mod tests;
