//! Публичные файлы офлайн-оболочки PWA.
//!
//! Регистрирует только два точных URL, не изменяя SPA и REST-маршруты.

use std::{fs, io::ErrorKind, path::Path};

use axum::{
    Router,
    body::Bytes,
    http::HeaderValue,
    response::{IntoResponse, Response},
    routing::get,
};

// Офлайн-оболочка содержит встроенные стили и подключает локальный JS.
const OFFLINE_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'";

const FILES: [(&str, &str, &str); 2] = [
    ("/offline.html", "offline.html", "text/html; charset=utf-8"),
    (
        "/pwa-register.js",
        "pwa-register.js",
        "application/javascript; charset=utf-8",
    ),
];

/// Регистрирует офлайн-страницу и скрипт регистрации PWA, если файлы существуют.
///
/// Содержимое читается при создании роутера и обновляется после перезапуска.
/// Отсутствующий файл не меняет набор маршрутов.
pub(super) fn mount_pwa_files<S>(mut router: Router<S>, directory: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    for (route, filename, content_type) in FILES {
        let path = directory.join(filename);
        let content = match fs::read(&path) {
            Ok(bytes) => Bytes::from(bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                tracing::error!(path = %path.display(), %error, "не удалось прочитать файл PWA");
                continue;
            }
        };
        tracing::info!(route, path = %path.display(), "включена раздача файла PWA");
        let is_html = route == "/offline.html";
        router = router.route(
            route,
            get(move || {
                let content = content.clone();
                async move {
                    let mut response: Response = (
                        [
                            ("content-type", content_type),
                            ("cache-control", "no-cache"),
                            ("x-content-type-options", "nosniff"),
                        ],
                        content,
                    )
                        .into_response();
                    if is_html {
                        let headers = response.headers_mut();
                        headers.insert(
                            "content-security-policy",
                            HeaderValue::from_static(OFFLINE_CSP),
                        );
                        headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
                        headers.insert(
                            "referrer-policy",
                            HeaderValue::from_static("strict-origin-when-cross-origin"),
                        );
                    }
                    response
                }
            }),
        );
    }
    router
}

#[cfg(test)]
mod tests;
