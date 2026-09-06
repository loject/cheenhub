//! Статичная страница завершения входа без токенов и клиентской сессии.

use axum::{
    extract::Query,
    http::header,
    response::{Html, IntoResponse},
};
use serde::Deserialize;

/// Безопасный статус страницы завершения; не содержит данных входа.
#[derive(Deserialize)]
pub(crate) struct ResultQuery {
    /// Значение `success` означает подтверждение Google.
    status: Option<String>,
}

/// Показывает следующий шаг после завершения входа в браузере.
pub(crate) async fn show(Query(query): Query<ResultQuery>) -> impl IntoResponse {
    let (title, message) = if query.status.as_deref() == Some("success") {
        (
            "Google подтверждён",
            "Вернись в приложение CheenHub, чтобы продолжить. Эту вкладку можно закрыть.",
        )
    } else {
        (
            "Вход не завершён",
            "Вернись в приложение CheenHub и начни вход через Google ещё раз. Эту вкладку можно закрыть.",
        )
    };
    let html = include_str!("desktop_result.html")
        .replace("{{title}}", title)
        .replace("{{message}}", message);
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "no-referrer"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
            ),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Html(html),
    )
}
