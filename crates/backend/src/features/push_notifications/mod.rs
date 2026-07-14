//! Вертикальная функция системных push-уведомлений.

pub(crate) mod application;
mod domain;
mod error;
mod fcm;
pub(crate) mod infrastructure;
mod transport;

use axum::{Router, routing::delete};

use crate::state::AppState;

pub(crate) use domain::DirectMessagePush;
pub(crate) use fcm::FcmClient;

/// Собирает REST-маршруты регистрации push-установок.
pub(crate) fn routes() -> Router<AppState> {
    Router::new().route(
        "/installations/{installation_id}",
        delete(transport::delete_installation).put(transport::upsert_installation),
    )
}
