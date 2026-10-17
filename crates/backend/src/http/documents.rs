//! Публичные служебные документы сайта с точными MIME-типами.
//!
//! Это только фиксированные маршруты; обработка SPA и произвольных файлов
//! остаётся в соседних HTTP-модулях.

use std::{fs, io::ErrorKind, path::Path};

use axum::{Router, body::Bytes, routing::get};

const DOCUMENTS: [(&str, &str, &str); 3] = [
    (
        "/manifest.webmanifest",
        "manifest.webmanifest",
        "application/manifest+json",
    ),
    ("/robots.txt", "robots.txt", "text/plain; charset=utf-8"),
    ("/sitemap.xml", "sitemap.xml", "application/xml"),
];

/// Подключает имеющиеся служебные документы из web-public.
///
/// При отсутствии файла соответствующий маршрут не добавляется. Содержимое
/// читается при старте и обновляется только после перезапуска backend.
pub(super) fn mount_documents<S>(mut router: Router<S>, directory: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    for (route, filename, content_type) in DOCUMENTS {
        let path = directory.join(filename);
        let content = match fs::read(&path) {
            Ok(bytes) => Bytes::from(bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                tracing::error!(path = %path.display(), %error, "не удалось прочитать служебный документ");
                continue;
            }
        };
        tracing::info!(route, path = %path.display(), "включена раздача служебного документа");
        router = router.route(
            route,
            get(move || {
                let content = content.clone();
                async move {
                    (
                        [
                            ("content-type", content_type),
                            ("cache-control", "no-cache"),
                            ("x-content-type-options", "nosniff"),
                        ],
                        content,
                    )
                }
            }),
        );
    }
    router
}

#[cfg(test)]
mod tests;
