//! Оболочка роутера REST API.

use axum::{Router, http::StatusCode, routing::get};

use crate::features::{auth, host_settings, images, push_notifications, servers, social};
use crate::realtime;
use crate::state::AppState;

/// Собирает роутер REST API.
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/images", images::routes())
        .nest("/host-settings", host_settings::routes())
        .nest("/push", push_notifications::routes())
        .nest("/friends", social::friend_routes())
        .nest("/direct", social::dm_routes())
        .nest("/direct-messages", social::dm_routes())
        .route("/realtime/ws", get(realtime::websocket::upgrade))
        .nest("/servers", servers::routes())
        .fallback(not_found)
}

/// Возвращает ответ по умолчанию для маршрутов, которые еще не реализованы.
pub(crate) async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

#[cfg(test)]
mod tests;

/// Проверка готовности backend принимать новые соединения.
///
/// Возвращает 200, пока процесс обслуживает трафик, и 503 после перехода в
/// фазу завершения. Docker Compose и внешний балансировщик используют этот
/// маршрут, чтобы не отправлять новые подключения в завершающийся процесс.
pub(crate) async fn health() -> StatusCode {
    let lifecycle = crate::lifecycle::lifecycle();
    if lifecycle.is_draining() {
        return StatusCode::SERVICE_UNAVAILABLE;
    }

    StatusCode::OK
}
