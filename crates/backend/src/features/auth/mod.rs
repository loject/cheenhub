//! Email/password authentication feature.

pub(crate) mod application;
mod domain;
pub(crate) mod error;
pub(crate) mod infrastructure;
pub(crate) mod security;
mod transport;
mod validation;

use axum::{
    Router,
    routing::{get, post},
};

use crate::http::AppState;

/// Builds authentication routes.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(transport::handlers::register))
        .route("/login", post(transport::handlers::login))
        .route("/refresh", post(transport::handlers::refresh))
        .route("/logout", post(transport::handlers::logout))
        .route("/me", get(transport::handlers::me))
}
