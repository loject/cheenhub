//! Общий потолок HTTP request body, соответствующий client_max_body_size Nginx.
//!
//! Более строгие ограничения извлекателей Axum и отдельных API остаются в силе.

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use tower_http::limit::RequestBodyLimitLayer;

const MAX_REQUEST_BODY_BYTES: usize = 9 * 1024 * 1024;

/// Ограничивает размеры тел всех HTTP-запросов до 9 МиБ.
///
/// Проверяет `Content-Length` заранее и ограничивает тело при чтении,
/// включая запросы, у которых размер изначально неизвестен.
pub(super) fn layer() -> RequestBodyLimitLayer {
    tracing::info!(
        max_body_bytes = MAX_REQUEST_BODY_BYTES,
        "включён общий лимит HTTP request body"
    );
    RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES)
}

/// Логирует отклонения по размеру, не записывая содержимое запросов.
pub(super) async fn log_rejection(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let response = next.run(request).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        tracing::warn!(
            %method,
            path,
            max_body_bytes = MAX_REQUEST_BODY_BYTES,
            "HTTP request body отклонён из-за превышения лимита"
        );
    }
    response
}

#[cfg(test)]
mod tests;
