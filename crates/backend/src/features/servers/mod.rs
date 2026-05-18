//! Server management feature.

mod application;
mod domain;
mod error;
pub(crate) mod infrastructure;
pub(crate) mod realtime;
mod transport;
mod validation;

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::state::AppState;

/// Builds server routes.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(transport::handlers::list).post(transport::handlers::create),
        )
        .route("/{server_id}", put(transport::handlers::update))
        .route(
            "/{server_id}/avatar",
            put(transport::handlers::update_avatar),
        )
        .route(
            "/{server_id}/invites",
            post(transport::handlers::create_invite),
        )
        .route(
            "/{server_id}/membership",
            delete(transport::handlers::leave),
        )
        .route(
            "/{server_id}/rooms",
            get(transport::handlers::list_rooms).post(transport::handlers::create_room),
        )
        .route(
            "/{server_id}/rooms/{room_id}",
            put(transport::handlers::update_room).delete(transport::handlers::delete_room),
        )
        .route("/invites/{code}", get(transport::handlers::invite_info))
        .route(
            "/invites/{code}/accept",
            post(transport::handlers::accept_invite),
        )
}
