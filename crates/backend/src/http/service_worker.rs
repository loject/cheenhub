//! Доставка service worker с запретом длительного кэширования скрипта.

use std::{fs, io::ErrorKind, path::Path};

use axum::{Router, body::Bytes, routing::get};

/// Подключает GET /sw.js из каталога собранного фронтенда.
///
/// При отсутствии файла не меняет роутер. Скрипт хранится в памяти до
/// перезапуска backend; браузер получает заголовки для повторной проверки.
pub(super) fn mount_service_worker<S>(router: Router<S>, directory: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let path = directory.join("sw.js");
    let script = match fs::read(&path) {
        Ok(content) => Bytes::from(content),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            tracing::debug!(path = %path.display(), "service worker отсутствует; маршрут /sw.js не включён");
            return router;
        }
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "не удалось загрузить service worker");
            return router;
        }
    };

    tracing::info!(path = %path.display(), "включена раздача service worker");
    router.route(
        "/sw.js",
        get(move || {
            let script = script.clone();
            async move {
                (
                    [
                        ("content-type", "application/javascript; charset=utf-8"),
                        ("cache-control", "no-cache, no-store, must-revalidate"),
                        ("service-worker-allowed", "/"),
                        ("x-content-type-options", "nosniff"),
                    ],
                    script,
                )
            }
        }),
    )
}

#[cfg(test)]
mod tests;
