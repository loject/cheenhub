//! Точечная раздача статических файлов без изменения API-маршрутов.
//!
//! Поддерживаются /index.html, /assets/* и /wasm/*; остальное остаётся за Nginx.

use std::{fs, io::ErrorKind, path::Path};

use axum::{
    Router,
    body::Bytes,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use tower_http::services::ServeDir;

// Политика соответствует SPA-политике текущего Nginx и разрешает Dioxus/WASM.
const APP_CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' https: wss:; worker-src 'self' blob:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'";

fn html_response(body: Bytes) -> Response {
    let mut response = (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response();
    let headers = response.headers_mut();
    headers.insert(
        "content-security-policy",
        axum::http::HeaderValue::from_static(APP_CSP),
    );
    headers.insert(
        "x-content-type-options",
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "x-frame-options",
        axum::http::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "referrer-policy",
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

#[derive(Clone)]
enum IndexFile {
    Ready(Bytes),
    Missing,
    Unreadable,
}

impl IndexFile {
    fn respond(self) -> Response {
        match self {
            Self::Ready(body) => html_response(body),
            Self::Missing => StatusCode::NOT_FOUND.into_response(),
            Self::Unreadable => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Регистрирует /index.html и SPA fallback при наличии собранного HTML.
///
/// Содержимое читается при создании роутера; после обновления фронтенда
/// процесс нужно перезапустить. При отсутствии index.html SPA fallback
/// не включается, а при отсутствии каталога роутер не меняется.
pub(super) fn mount_index<S>(router: Router<S>, directory: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !directory.is_dir() {
        tracing::debug!(
            directory = %directory.display(),
            "каталог фронтенда отсутствует; HTTP API работает без статики"
        );
        return router;
    }

    let path = directory.join("index.html");
    let file = match fs::read(&path) {
        Ok(bytes) => IndexFile::Ready(Bytes::from(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            tracing::warn!(path = %path.display(), "index.html отсутствует; маршрут вернёт 404");
            IndexFile::Missing
        }
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "не удалось прочитать index.html");
            IndexFile::Unreadable
        }
    };
    let spa_html = match &file {
        IndexFile::Ready(body) => Some(body.clone()),
        IndexFile::Missing | IndexFile::Unreadable => None,
    };
    tracing::info!(directory = %directory.display(), "включена раздача /index.html");

    let router = router.route(
        "/index.html",
        get(move || {
            let file = file.clone();
            async move { file.respond() }
        }),
    );
    let Some(html) = spa_html else {
        return router;
    };
    tracing::info!("включён SPA fallback для HTML-навигации");
    router.fallback(move |method: axum::http::Method, uri: axum::http::Uri| {
        let html = html.clone();
        async move {
            if !is_spa_route(&method, uri.path()) {
                return StatusCode::NOT_FOUND.into_response();
            }
            let body = if method == axum::http::Method::HEAD {
                Bytes::new()
            } else {
                html
            };
            html_response(body)
        }
    })
}

fn is_spa_route(method: &axum::http::Method, path: &str) -> bool {
    if method != axum::http::Method::GET && method != axum::http::Method::HEAD {
        return false;
    }
    if path == "/" || path.split('/').any(|segment| segment.contains('.')) {
        return false;
    }
    const RESERVED: [&str; 6] = [
        "/api",
        "/health",
        "/assets",
        "/wasm",
        "/icons",
        "/screenshots",
    ];
    !RESERVED.iter().any(|prefix| {
        path == *prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|tail| tail.starts_with('/'))
    })
}

/// Подключает каталоги ресурсов фронтенда без SPA fallback.
///
/// Регистрируются только существующие каталоги /assets/ и /wasm/.
/// Файлы читаются с диска при запросе, поэтому изменения ресурсов видны
/// без перезапуска. Успешные ответы кэшируются на год как immutable,
/// а ошибки не получают длительного кэширования.
pub(super) fn mount_asset_dirs<S>(mut router: Router<S>, directory: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let assets = directory.join("assets");
    if assets.is_dir() {
        tracing::info!(directory = %assets.display(), "включена раздача /assets/");
        router = router.nest("/assets", immutable_files(assets));
    }

    let wasm = directory.join("wasm");
    if wasm.is_dir() {
        tracing::info!(directory = %wasm.display(), "включена раздача /wasm/");
        router = router.nest("/wasm", immutable_files(wasm));
    }

    router
}

// Кэширование ограничено вложенным роутером: API и HTML его не наследуют.
fn immutable_files<S>(directory: std::path::PathBuf) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .fallback_service(ServeDir::new(directory))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                if response.status().is_success() || response.status() == StatusCode::NOT_MODIFIED {
                    response.headers_mut().insert(
                        header::CACHE_CONTROL,
                        axum::http::HeaderValue::from_static("public, max-age=31536000, immutable"),
                    );
                }
                response
            },
        ))
}

#[cfg(test)]
mod tests;
