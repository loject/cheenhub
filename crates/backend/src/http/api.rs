//! REST API router shell.

use axum::{Router, http::StatusCode};

use crate::features::auth;
use crate::http::AppState;

/// Builds the REST API router.
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .fallback(not_found)
}

/// Returns the default response for routes that are not implemented yet.
pub(crate) async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
