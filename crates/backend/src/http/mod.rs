//! HTTP router setup.

mod api;

use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

/// Builds the backend HTTP router.
pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api", api::router())
        .fallback(api::not_found)
        .with_state(state)
        .layer(CorsLayer::permissive()) // TODO: fix for prod
        .layer(TraceLayer::new_for_http())
}
