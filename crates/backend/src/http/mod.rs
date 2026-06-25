//! Настройка HTTP-роутера.

mod api;

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

/// Собирает HTTP-роутер бэкенда.
pub(crate) fn router(state: AppState) -> Router {
    let cors = cors_layer(&state.cheenhub_client_base_url);
    Router::new()
        .nest("/api", api::router())
        .fallback(api::not_found)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// Строит CORS-слой, ограниченный известным origin клиента.
fn cors_layer(client_base_url: &str) -> CorsLayer {
    let origin = client_base_url
        .trim_end_matches('/')
        .parse::<HeaderValue>();
    match origin {
        Ok(origin) => CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        Err(error) => {
            tracing::error!(
                %error,
                client_base_url,
                "invalid CHEENHUB_CLIENT_BASE_URL; serving API without cross-origin access"
            );
            CorsLayer::new()
        }
    }
}
