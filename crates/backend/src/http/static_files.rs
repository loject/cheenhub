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

#[derive(Clone)]
enum IndexFile {
    Ready(Bytes),
    Missing,
    Unreadable,
}

impl IndexFile {
    fn respond(self) -> Response {
        match self {
            Self::Ready(body) => {
                ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body).into_response()
            }
            Self::Missing => StatusCode::NOT_FOUND.into_response(),
            Self::Unreadable => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Регистрирует GET /index.html, только если каталог статики существует.
///
/// Содержимое читается однократно при создании роутера; после обновления
/// фронтенда процесс нужно перезапустить. Если каталога нет, роутер не меняется.
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
    tracing::info!(directory = %directory.display(), "включена раздача /index.html");

    router.route(
        "/index.html",
        get(move || {
            let file = file.clone();
            async move { file.respond() }
        }),
    )
}

/// Подключает каталоги ресурсов фронтенда без SPA fallback.
///
/// Регистрируются только существующие каталоги /assets/ и /wasm/.
/// Файлы читаются с диска при запросе, поэтому изменения ресурсов видны
/// без перезапуска. Отсутствующие файлы возвращают 404.
pub(super) fn mount_asset_dirs<S>(mut router: Router<S>, directory: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let assets = directory.join("assets");
    if assets.is_dir() {
        tracing::info!(directory = %assets.display(), "включена раздача /assets/");
        router = router.nest_service("/assets", ServeDir::new(assets));
    }

    let wasm = directory.join("wasm");
    if wasm.is_dir() {
        tracing::info!(directory = %wasm.display(), "включена раздача /wasm/");
        router = router.nest_service("/wasm", ServeDir::new(wasm));
    }

    router
}

#[cfg(test)]
mod tests;
