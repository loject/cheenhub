//! Глобальные настройки экземпляра CheenHub и права владельцев хоста.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod email_delivery;
pub(crate) mod infrastructure;
pub(crate) mod metrics_monitor;
mod transport;

use axum::Router;

use crate::state::AppState;

/// Возвращает REST-маршруты настроек хоста.
pub(crate) fn routes() -> Router<AppState> {
    transport::routes()
}
