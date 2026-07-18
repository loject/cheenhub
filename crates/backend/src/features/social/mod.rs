//! Функция друзей и личных сообщений.

mod application;
mod domain;
mod error;
pub(crate) mod infrastructure;
pub(crate) mod realtime;
mod support;
mod transport;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
};

use crate::state::AppState;

pub(crate) use application::{
    DirectMessageVoiceAccess, direct_message_voice_access, direct_message_voice_accesses_for_user,
    direct_message_voice_user_ids,
};
pub(crate) use error::SocialError;

#[cfg(test)]
pub(crate) use application::{accept_friend_request, open_dm_conversation, send_friend_request};

/// Собирает маршруты друзей.
pub(crate) fn friend_routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(transport::search_users))
        .route("/", get(transport::list_friends))
        .route("/requests/incoming", get(transport::list_incoming_requests))
        .route("/requests/outgoing", get(transport::list_outgoing_requests))
        .route("/requests", post(transport::send_friend_request))
        .route(
            "/requests/{request_id}/accept",
            post(transport::accept_friend_request),
        )
        .route(
            "/requests/{request_id}/decline",
            post(transport::decline_friend_request),
        )
        .route(
            "/requests/{request_id}/cancel",
            post(transport::cancel_friend_request),
        )
        .route("/{friend_user_id}", delete(transport::delete_friend))
}

/// Собирает маршруты личных сообщений.
pub(crate) fn dm_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/conversations",
            get(transport::list_dm_conversations).post(transport::open_dm_conversation),
        )
        .route(
            "/conversations/{conversation_id}/messages",
            get(transport::list_dm_messages).post(transport::send_dm_message),
        )
        .route(
            "/conversations/{conversation_id}/images",
            post(transport::upload_dm_image).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route(
            "/conversations/{conversation_id}/images/{image_id}",
            get(transport::dm_image),
        )
        .route(
            "/conversations/{conversation_id}/read",
            post(transport::mark_dm_conversation_read),
        )
}
