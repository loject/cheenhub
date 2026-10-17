//! Главная страница CheenHub: отдельный HTML и строгая политика браузера.

use std::{fs, io::ErrorKind, path::Path};

use axum::{Router, body::Bytes, routing::get};

const LANDING_CSP: &str = "default-src 'self'; script-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'";

/// Добавляет GET / из web-public/landing.html, не меняя остальные маршруты.
///
/// HTML загружается однократно при запуске. Отсутствующий файл оставляет
/// прежний fallback, а обновлённый файл становится доступен после рестарта.
pub(super) fn mount_landing<S>(router: Router<S>, directory: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if !directory.is_dir() {
        return router;
    }

    let path = directory.join("landing.html");
    let body = match fs::read(&path) {
        Ok(contents) => Bytes::from(contents),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            tracing::warn!(path = %path.display(), "лендинг не найден; маршрут / не включён");
            return router;
        }
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "не удалось загрузить лендинг");
            return router;
        }
    };

    tracing::info!(path = %path.display(), "включена раздача лендинга по /");
    router.route(
        "/",
        get(move || {
            let body = body.clone();
            async move {
                (
                    [
                        ("content-type", "text/html; charset=utf-8"),
                        ("cache-control", "no-cache"),
                        ("content-security-policy", LANDING_CSP),
                        ("x-content-type-options", "nosniff"),
                        ("x-frame-options", "DENY"),
                        ("referrer-policy", "strict-origin-when-cross-origin"),
                    ],
                    body,
                )
            }
        }),
    )
}

#[cfg(test)]
mod tests;
