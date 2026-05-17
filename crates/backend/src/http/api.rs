//! REST API router shell.

use axum::{Router, http::StatusCode, routing::get};

use crate::features::{auth, images, servers};
use crate::realtime;
use crate::state::AppState;

/// Builds the REST API router.
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/images", images::routes())
        .route("/realtime/ws", get(realtime::websocket::upgrade))
        .nest("/servers", servers::routes())
        .fallback(not_found)
}

/// Returns the default response for routes that are not implemented yet.
pub(crate) async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
