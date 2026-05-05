//! HTTP router setup.

mod api;

use axum::Router;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::features::auth::infrastructure::AuthStore;
use crate::features::auth::security::keys::AuthKeys;

/// Shared HTTP application state.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Authentication storage backend.
    pub(crate) auth_store: Arc<dyn AuthStore>,
    /// Access JWT signing keys.
    pub(crate) auth_keys: AuthKeys,
    /// Access JWT lifetime in minutes.
    pub(crate) access_token_lifetime_minutes: i64,
    /// Refresh token lifetime in days.
    pub(crate) refresh_token_lifetime_days: i64,
}

/// Builds the backend HTTP router.
pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api", api::router())
        .fallback(api::not_found)
        .with_state(state)
        .layer(CorsLayer::permissive()) // TODO: fix for prod
        .layer(TraceLayer::new_for_http())
}
