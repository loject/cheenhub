//! HTTP router setup.

mod api;

use axum::Router;
use tower_http::trace::TraceLayer;

/// Builds the backend HTTP router.
pub(crate) fn router() -> Router {
    Router::new()
        .nest("/api", api::router())
        .fallback(api::not_found)
        .layer(TraceLayer::new_for_http())
}
