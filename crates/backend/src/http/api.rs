//! Оболочка роутера REST API.

use axum::{Router, http::StatusCode, routing::get};

use crate::features::{auth, images, servers};
use crate::realtime;
use crate::state::AppState;

/// Собирает роутер REST API.
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/images", images::routes())
        .route("/realtime/ws", get(realtime::websocket::upgrade))
        .nest("/servers", servers::routes())
        .fallback(not_found)
}

/// Возвращает ответ по умолчанию для маршрутов, которые еще не реализованы.
pub(crate) async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
