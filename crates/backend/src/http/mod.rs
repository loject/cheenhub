//! Настройка HTTP-роутера.

mod api;

use axum::Router;
use axum::http::{HeaderValue, Method, Uri, header, request::Parts};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

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

/// Строит CORS-слой.
fn cors_layer(client_base_url: &str) -> CorsLayer {
    let configured_origin = match client_base_url.trim_end_matches('/').parse::<HeaderValue>() {
        Ok(origin) => Some(origin),
        Err(error) => {
            tracing::error!(
                %error,
                client_base_url,
                "invalid CHEENHUB_CLIENT_BASE_URL; ignoring configured client origin"
            );
            None
        }
    };

    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(
            move |origin: &HeaderValue, _request_parts: &Parts| {
                if configured_origin.as_ref() == Some(origin) {
                    return true;
                }

                is_allowed_lan_origin(origin)
            },
        ))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

/// Проверяет origin вида:
/// http://192.168.1.123:3000
/// http://192.168.2.50:8080
/// http://127.0.0.1:5173
/// https://127.0.0.1:5173
fn is_allowed_lan_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };

    let Ok(uri) = origin.parse::<Uri>() else {
        return false;
    };

    let Some(scheme) = uri.scheme_str() else {
        return false;
    };

    if scheme != "http" && scheme != "https" {
        return false;
    }

    let Some(host) = uri.host() else {
        return false;
    };

    host == "127.0.0.1" || host.starts_with("192.168.1.") || host.starts_with("192.168.2.")
}
