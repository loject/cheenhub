//! Тестовые помощники связи auth-сессий с realtime-транспортами.

use cheenhub_contracts::rest::AuthResponse;
use uuid::Uuid;

use crate::features::auth::security::jwt;
use crate::state::AppState;

/// Регистрирует тестовый realtime-транспорт для auth-ответа.
pub(super) async fn register_test_session(
    state: &AppState,
    auth: &AuthResponse,
) -> tokio::sync::watch::Receiver<bool> {
    let claims = jwt::verify_access_token(&state.auth_keys, &auth.access_token)
        .expect("test access token should be valid");
    let user_id = Uuid::parse_str(&auth.user.id).expect("test user id should be a uuid");
    let auth_session_id =
        Uuid::parse_str(&claims.session_id).expect("test auth session id should be a uuid");
    state
        .realtime_hub
        .register_test_session(user_id, auth_session_id)
        .await
}
