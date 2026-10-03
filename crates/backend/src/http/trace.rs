//! HTTP-трассировка без секретов из query-параметров callback и ссылок входа.

use axum::http::Request;
use tracing::Span;

/// Записывает путь запроса без query, в котором могут находиться OAuth code и state.
pub(super) fn request_span<B>(request: &Request<B>) -> Span {
    tracing::debug_span!(
        "http_request",
        method = %request.method(),
        path = request.uri().path(),
        version = ?request.version(),
    )
}

#[cfg(test)]
mod tests;
