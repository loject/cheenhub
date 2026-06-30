//! Функция аутентификации по email и паролю.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod email;
pub(crate) mod error;
pub(crate) mod infrastructure;
pub(crate) mod security;
mod transport;
mod validation;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};

use crate::state::AppState;

/// Собирает маршруты аутентификации.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(transport::handlers::register))
        .route("/login", post(transport::handlers::login))
        .route(
            "/password-reset/request",
            post(transport::handlers::request_password_reset),
        )
        .route(
            "/password-reset/confirm",
            post(transport::handlers::confirm_password_reset),
        )
        .route("/refresh", post(transport::handlers::refresh))
        .route("/logout", post(transport::handlers::logout))
        .route(
            "/sessions",
            get(transport::handlers::active_sessions).delete(transport::handlers::revoke_sessions),
        )
        .route(
            "/sessions/{session_id}",
            axum::routing::delete(transport::handlers::revoke_session),
        )
        .route(
            "/me",
            get(transport::handlers::me).patch(transport::handlers::update_current_user),
        )
        .route(
            "/me/password",
            post(transport::handlers::change_current_user_password),
        )
        .route(
            "/me/avatar",
            put(transport::handlers::update_current_user_avatar)
                .layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/oauth/google/start",
            post(transport::handlers::start_google_oauth),
        )
        .route(
            "/oauth/google/callback",
            get(transport::handlers::google_oauth_callback),
        )
        .route(
            "/oauth/google/complete",
            post(transport::handlers::complete_google_oauth),
        )
        .route(
            "/oauth/google/register",
            post(transport::handlers::register_with_google_oauth),
        )
        .route(
            "/linked-accounts",
            get(transport::handlers::linked_accounts),
        )
        .route(
            "/linked-accounts/google/unlink",
            post(transport::handlers::unlink_google),
        )
}
