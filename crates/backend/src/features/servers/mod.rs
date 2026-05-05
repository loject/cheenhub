//! Server management feature.

mod application;
mod domain;
mod error;
pub(crate) mod infrastructure;
mod transport;
mod validation;

use axum::{
    Router,
    routing::{get, post},
};

use crate::http::AppState;

/// Builds server routes.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(transport::handlers::list).post(transport::handlers::create),
        )
        .route(
            "/{server_id}/invites",
            post(transport::handlers::create_invite),
        )
}
