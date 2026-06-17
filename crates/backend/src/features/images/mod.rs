//! Функция обработки изображений и публичной доставки.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
mod transport;

use axum::Router;

use crate::state::AppState;

/// Собирает маршруты публичных изображений.
pub(crate) fn routes() -> Router<AppState> {
    transport::routes()
}
