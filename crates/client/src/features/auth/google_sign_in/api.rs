//! REST-вызовы нативного входа через Google.

use cheenhub_contracts::rest::{GoogleNativeAuthCompleteRequest, GoogleNativeAuthStartResponse};
use serde_json::Value;

use crate::features::auth::api::{OAuthCompletion, parse_oauth_completion, post, read_error};

/// Запрашивает одноразовый challenge для нативного входа Google.
pub(super) async fn start() -> Result<GoogleNativeAuthStartResponse, String> {
    let response = post("/auth/oauth/google/native/start")
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;
    if response.status().is_success() {
        response
            .json()
            .await
            .map_err(|_| "Не удалось прочитать параметры входа Google.".to_owned())
    } else {
        Err(read_error(response).await)
    }
}

/// Передаёт Google ID Token backend для проверки и завершения входа.
pub(super) async fn complete(
    challenge: String,
    id_token: String,
) -> Result<OAuthCompletion, String> {
    let response = post("/auth/oauth/google/native/complete")
        .json(&GoogleNativeAuthCompleteRequest {
            challenge,
            id_token,
        })
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;
    if !response.status().is_success() {
        return Err(read_error(response).await);
    }
    let value = response
        .json::<Value>()
        .await
        .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())?;
    parse_oauth_completion(value)
}
