//! Поток нативной авторизации Google для Android.

use cheenhub_contracts::rest::{
    GoogleNativeAuthCompleteRequest, GoogleNativeAuthStartResponse, OAuthCompleteRequest,
    OAuthCompleteResponse,
};
use chrono::{Duration, Utc};
use tracing::{error, info, warn};

use super::google::{google_client_id, verify_google_id_token};
use super::oauth::{complete_google_oauth, create_google_handoff};
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;
use crate::state::AppState;

const GOOGLE_PROVIDER: &str = "google";
const OAUTH_FLOW_LOGIN: &str = "login";
const OAUTH_FLOW_NATIVE_LOGIN: &str = "native_login";

/// Создает одноразовый challenge для нативного Google Sign-In.
pub(crate) async fn start_google_native_auth(
    state: &AppState,
) -> Result<GoogleNativeAuthStartResponse, AuthError> {
    let server_client_id = google_client_id(state)?;
    let now = Utc::now();
    let challenge = refresh_token::generate();
    let nonce = refresh_token::generate();
    let expires_at = now + Duration::minutes(state.oauth_state_lifetime_minutes);
    state
        .auth_store
        .insert_oauth_state(
            refresh_token::hash(&challenge),
            nonce.clone(),
            OAUTH_FLOW_NATIVE_LOGIN.to_owned(),
            None,
            now,
            expires_at,
        )
        .await
        .map_err(|error| {
            error!(
                provider = GOOGLE_PROVIDER,
                flow_kind = OAUTH_FLOW_NATIVE_LOGIN,
                %expires_at,
                %error,
                "failed to persist google native auth challenge"
            );
            AuthError::Internal(error)
        })?;
    info!(
        provider = GOOGLE_PROVIDER,
        flow_kind = OAUTH_FLOW_NATIVE_LOGIN,
        %expires_at,
        "started google native auth flow"
    );

    Ok(GoogleNativeAuthStartResponse {
        challenge,
        nonce,
        server_client_id,
    })
}

/// Проверяет нативный Google ID Token и завершает вход в CheenHub.
pub(crate) async fn complete_google_native_auth(
    state: &AppState,
    request: GoogleNativeAuthCompleteRequest,
    user_agent: Option<String>,
) -> Result<OAuthCompleteResponse, AuthError> {
    if request.id_token.len() > 16 * 1024 {
        warn!(
            provider = GOOGLE_PROVIDER,
            flow_kind = OAUTH_FLOW_NATIVE_LOGIN,
            "rejected oversized google native id token"
        );
        return Err(AuthError::BadRequest(
            "Ответ Google имеет недопустимый размер.".to_owned(),
        ));
    }
    let now = Utc::now();
    let Some(challenge) = state
        .auth_store
        .consume_oauth_state(&refresh_token::hash(&request.challenge), now)
        .await
        .map_err(AuthError::Internal)?
    else {
        warn!(
            provider = GOOGLE_PROVIDER,
            flow_kind = OAUTH_FLOW_NATIVE_LOGIN,
            "rejected expired or already consumed google native auth challenge"
        );
        return Err(AuthError::Unauthorized(
            "Вход через Google истек. Попробуй еще раз.".to_owned(),
        ));
    };
    if challenge.flow_kind != OAUTH_FLOW_NATIVE_LOGIN || challenge.user_id.is_some() {
        warn!(
            provider = GOOGLE_PROVIDER,
            flow_kind = %challenge.flow_kind,
            "rejected google native auth challenge with unexpected flow"
        );
        return Err(AuthError::Unauthorized(
            "Этот запрос входа через Google недействителен.".to_owned(),
        ));
    }

    let client_id = google_client_id(state)?;
    let identity =
        match verify_google_id_token(&client_id, &request.id_token, &challenge.nonce).await {
            Ok(identity) => identity,
            Err(error) => {
                match &error {
                    AuthError::Internal(cause) => error!(
                        provider = GOOGLE_PROVIDER,
                        flow_kind = OAUTH_FLOW_NATIVE_LOGIN,
                        %cause,
                        "google native auth could not verify id token because jwks setup failed"
                    ),
                    _ => warn!(
                        provider = GOOGLE_PROVIDER,
                        flow_kind = OAUTH_FLOW_NATIVE_LOGIN,
                        "rejected invalid google native id token"
                    ),
                }
                return Err(error);
            }
        };
    let handoff_code = create_google_handoff(state, OAUTH_FLOW_LOGIN, None, &identity, now).await?;
    let response =
        complete_google_oauth(state, OAuthCompleteRequest { handoff_code }, user_agent).await?;
    info!(
        provider = GOOGLE_PROVIDER,
        flow_kind = OAUTH_FLOW_NATIVE_LOGIN,
        "completed google native auth flow"
    );

    Ok(response)
}
