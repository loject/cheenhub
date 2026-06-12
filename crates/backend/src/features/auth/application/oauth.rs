//! Google OAuth application flows.
use anyhow::anyhow;
use cheenhub_contracts::rest::*;
use chrono::{Duration, Utc};
use tracing::{error, info, warn};
use url::Url;
use uuid::Uuid;

use super::google::{GoogleIdentity, exchange_google_code, frontend_oauth_url, google_config};
use super::{create_auth_response, expired_session, map_insert_user_error, me};
use crate::features::auth::domain::*;
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;
use crate::features::auth::validation;
use crate::state::AppState;

const GOOGLE_PROVIDER: &str = "google";
const OAUTH_FLOW_LOGIN: &str = "login";
const OAUTH_FLOW_LINK: &str = "link";
const HANDOFF_AUTHENTICATED: &str = "authenticated";
const HANDOFF_LINKED: &str = "linked";
const HANDOFF_REGISTRATION_REQUIRED: &str = "registration_required";

/// Starts a Google OAuth login or account linking flow.
pub(crate) async fn start_google_oauth(
    state: &AppState,
    access_token: Option<&str>,
    request: OAuthStartRequest,
) -> Result<OAuthStartResponse, AuthError> {
    let config = google_config(state)?;
    let now = Utc::now();
    let (flow_kind, user_id) = match request.flow {
        OAuthFlow::Login => (OAUTH_FLOW_LOGIN.to_owned(), None),
        OAuthFlow::Link => {
            let token = access_token
                .ok_or_else(|| AuthError::Unauthorized("Войди, чтобы продолжить.".to_owned()))?;
            let user = me(state, token).await?;
            let user_id = Uuid::parse_str(&user.id).map_err(|_| expired_session())?;
            (OAUTH_FLOW_LINK.to_owned(), Some(user_id))
        }
    };

    let state_value = refresh_token::generate();
    let nonce = refresh_token::generate();
    let expires_at = now + Duration::minutes(state.oauth_state_lifetime_minutes);
    state
        .auth_store
        .insert_oauth_state(
            refresh_token::hash(&state_value),
            nonce.clone(),
            flow_kind.clone(),
            user_id,
            now,
            expires_at,
        )
        .await
        .map_err(|error| {
            error!(
                provider = GOOGLE_PROVIDER,
                flow_kind,
                ?user_id,
                %expires_at,
                %error,
                "failed to persist google oauth state; ensure database migrations are applied and oauth_states table exists"
            );
            AuthError::Internal(error)
        })?;

    info!(
        provider = GOOGLE_PROVIDER,
        flow_kind,
        ?user_id,
        %expires_at,
        "started google oauth flow"
    );

    let mut url =
        Url::parse("https://accounts.google.com/o/oauth2/v2/auth").map_err(anyhow::Error::from)?;
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", &state_value)
        .append_pair("nonce", &nonce)
        .append_pair("prompt", "select_account");

    Ok(OAuthStartResponse {
        authorization_url: url.to_string(),
    })
}

/// Handles the Google OAuth callback and returns a frontend redirect URL.
pub(crate) async fn google_oauth_callback_url(
    state: &AppState,
    code: Option<String>,
    state_value: Option<String>,
    error: Option<String>,
) -> String {
    match google_oauth_callback(state, code, state_value, error).await {
        Ok(code) => frontend_oauth_url(state, &[("code", code.as_str())]),
        Err(error) => {
            let message = error
                .user_message()
                .unwrap_or("Не удалось войти через Google. Попробуй еще раз.");
            warn!(?error, error_message = %message, "google oauth callback failed");
            frontend_oauth_url(state, &[("error", message)])
        }
    }
}

/// Completes a frontend OAuth handoff.
pub(crate) async fn complete_google_oauth(
    state: &AppState,
    request: OAuthCompleteRequest,
    user_agent: Option<String>,
) -> Result<OAuthCompleteResponse, AuthError> {
    let now = Utc::now();
    let code_hash = refresh_token::hash(&request.handoff_code);
    let Some(handoff) = state
        .auth_store
        .find_active_oauth_handoff(&code_hash, now)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(AuthError::Unauthorized(
            "Вход через Google истек. Попробуй еще раз.".to_owned(),
        ));
    };

    match handoff.kind.as_str() {
        HANDOFF_AUTHENTICATED => {
            let user_id = handoff
                .user_id
                .ok_or_else(|| AuthError::Internal(anyhow!("oauth auth handoff without user")))?;
            let user = find_user_or_expired(state, &user_id).await?;
            state
                .auth_store
                .consume_oauth_handoff(&handoff.id, now)
                .await
                .map_err(AuthError::Internal)?;
            Ok(OAuthCompleteResponse::Authenticated {
                auth: create_auth_response(state, &user, user_agent.as_deref()).await?,
            })
        }
        HANDOFF_LINKED => {
            let user_id = handoff
                .user_id
                .ok_or_else(|| AuthError::Internal(anyhow!("oauth link handoff without user")))?;
            let account = state
                .auth_store
                .find_oauth_account_for_user(GOOGLE_PROVIDER, &user_id)
                .await
                .map_err(AuthError::Internal)?
                .ok_or_else(|| AuthError::Internal(anyhow!("linked oauth account missing")))?;
            state
                .auth_store
                .consume_oauth_handoff(&handoff.id, now)
                .await
                .map_err(AuthError::Internal)?;
            Ok(OAuthCompleteResponse::Linked {
                account: linked_account(&account),
            })
        }
        HANDOFF_REGISTRATION_REQUIRED => {
            let intent_id = handoff.registration_intent_id.ok_or_else(|| {
                AuthError::Internal(anyhow!("oauth registration handoff without intent"))
            })?;
            let intent = state
                .auth_store
                .find_active_oauth_registration_intent(&intent_id, now)
                .await
                .map_err(AuthError::Internal)?
                .ok_or_else(|| {
                    AuthError::Unauthorized("Регистрация через Google истекла.".to_owned())
                })?;
            Ok(OAuthCompleteResponse::RegistrationRequired {
                registration_token: request.handoff_code,
                email: intent.email,
                display_name: intent.display_name,
            })
        }
        _ => Err(AuthError::Internal(anyhow!("unknown oauth handoff kind"))),
    }
}

/// Completes registration for a verified Google OAuth identity.
pub(crate) async fn register_with_google_oauth(
    state: &AppState,
    request: OAuthRegistrationRequest,
    user_agent: Option<String>,
) -> Result<AuthResponse, AuthError> {
    if !request.accepts_policies {
        return Err(AuthError::BadRequest(
            "Нужно принять правила сервиса.".to_owned(),
        ));
    }
    let nickname = request.nickname.trim().to_owned();
    if !validation::is_valid_nickname(&nickname) {
        return Err(AuthError::BadRequest(
            "Никнейм должен быть длиной 3-32 символа и содержать латиницу, цифры или _.".to_owned(),
        ));
    }

    let now = Utc::now();
    let code_hash = refresh_token::hash(&request.registration_token);
    let Some(handoff) = state
        .auth_store
        .find_active_oauth_handoff(&code_hash, now)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(AuthError::Unauthorized(
            "Регистрация через Google истекла.".to_owned(),
        ));
    };
    if handoff.kind != HANDOFF_REGISTRATION_REQUIRED {
        return Err(AuthError::BadRequest(
            "Этот OAuth токен нельзя использовать для регистрации.".to_owned(),
        ));
    }
    let intent_id = handoff
        .registration_intent_id
        .ok_or_else(|| AuthError::Internal(anyhow!("registration handoff without intent")))?;
    let intent = state
        .auth_store
        .find_active_oauth_registration_intent(&intent_id, now)
        .await
        .map_err(AuthError::Internal)?
        .ok_or_else(|| AuthError::Unauthorized("Регистрация через Google истекла.".to_owned()))?;

    if state
        .auth_store
        .find_oauth_account_by_subject(GOOGLE_PROVIDER, &intent.provider_subject)
        .await
        .map_err(AuthError::Internal)?
        .is_some()
    {
        return Err(AuthError::Conflict(
            "Этот Google аккаунт уже привязан.".to_owned(),
        ));
    }

    let user = state
        .auth_store
        .insert_user(
            nickname,
            intent.email.clone(),
            intent.email.to_lowercase(),
            None,
            now,
        )
        .await
        .map_err(map_insert_user_error)?;
    state
        .auth_store
        .insert_oauth_account(
            &user.id,
            GOOGLE_PROVIDER.to_owned(),
            intent.provider_subject,
            intent.email,
            intent.display_name,
            now,
        )
        .await
        .map_err(map_oauth_link_error)?;
    state
        .auth_store
        .consume_oauth_registration_intent(&intent_id, now)
        .await
        .map_err(AuthError::Internal)?;
    state
        .auth_store
        .consume_oauth_handoff(&handoff.id, now)
        .await
        .map_err(AuthError::Internal)?;

    create_auth_response(state, &user, user_agent.as_deref()).await
}

/// Lists external accounts linked to the current user.
pub(crate) async fn linked_accounts(
    state: &AppState,
    access_token: &str,
) -> Result<LinkedAccountsResponse, AuthError> {
    let user = me(state, access_token).await?;
    let user_id = Uuid::parse_str(&user.id).map_err(|_| expired_session())?;
    let accounts = state
        .auth_store
        .list_oauth_accounts(&user_id)
        .await
        .map_err(AuthError::Internal)?
        .iter()
        .map(linked_account)
        .collect();

    Ok(LinkedAccountsResponse { accounts })
}

/// Unlinks Google from the current user when another login method remains.
pub(crate) async fn unlink_google(
    state: &AppState,
    access_token: &str,
) -> Result<LinkedAccountsResponse, AuthError> {
    let user = me(state, access_token).await?;
    let user_id = Uuid::parse_str(&user.id).map_err(|_| expired_session())?;
    let user = find_user_or_expired(state, &user_id).await?;
    if user.password_hash.is_none() {
        return Err(AuthError::BadRequest(
            "Сначала добавь пароль, чтобы не потерять доступ к аккаунту.".to_owned(),
        ));
    }
    state
        .auth_store
        .delete_oauth_account(GOOGLE_PROVIDER, &user_id)
        .await
        .map_err(AuthError::Internal)?;
    linked_accounts(state, access_token).await
}

async fn google_oauth_callback(
    state: &AppState,
    code: Option<String>,
    state_value: Option<String>,
    error: Option<String>,
) -> Result<String, AuthError> {
    if let Some(error) = error {
        return Err(AuthError::BadRequest(format!(
            "Google OAuth вернул ошибку: {error}"
        )));
    }
    let code = code.ok_or_else(|| AuthError::BadRequest("Google не вернул код.".to_owned()))?;
    let state_value =
        state_value.ok_or_else(|| AuthError::BadRequest("Google не вернул state.".to_owned()))?;
    let now = Utc::now();
    let Some(oauth_state) = state
        .auth_store
        .consume_oauth_state(&refresh_token::hash(&state_value), now)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(AuthError::Unauthorized(
            "Вход через Google истек. Попробуй еще раз.".to_owned(),
        ));
    };
    let config = google_config(state)?;
    let identity = exchange_google_code(&config, &code, &oauth_state.nonce).await?;

    let (kind, user_id, registration_intent_id) = match oauth_state.flow_kind.as_str() {
        OAUTH_FLOW_LINK => {
            let user_id = oauth_state
                .user_id
                .ok_or_else(|| AuthError::Internal(anyhow!("link oauth state without user")))?;
            link_google_identity(state, &user_id, &identity, now).await?;
            info!(%user_id, "linked google oauth account");
            (HANDOFF_LINKED.to_owned(), Some(user_id), None)
        }
        OAUTH_FLOW_LOGIN => match state
            .auth_store
            .find_oauth_account_by_subject(GOOGLE_PROVIDER, &identity.subject)
            .await
            .map_err(AuthError::Internal)?
        {
            Some(account) => {
                info!(user_id = %account.user_id, "accepted google oauth login");
                (
                    HANDOFF_AUTHENTICATED.to_owned(),
                    Some(account.user_id),
                    None,
                )
            }
            None => {
                let user_id = if let Some(user) = state
                    .auth_store
                    .find_user_by_email(&identity.email.to_lowercase())
                    .await
                    .map_err(AuthError::Internal)?
                {
                    link_google_identity(state, &user.id, &identity, now).await?;
                    info!(user_id = %user.id, "auto-linked google oauth account by verified email");
                    Some(user.id)
                } else {
                    None
                };
                if let Some(user_id) = user_id {
                    (HANDOFF_AUTHENTICATED.to_owned(), Some(user_id), None)
                } else {
                    let intent = state
                        .auth_store
                        .insert_oauth_registration_intent(
                            GOOGLE_PROVIDER.to_owned(),
                            identity.subject,
                            identity.email,
                            identity.display_name,
                            now,
                            now + Duration::minutes(state.oauth_registration_lifetime_minutes),
                        )
                        .await
                        .map_err(AuthError::Internal)?;
                    info!(registration_intent_id = %intent.id, "created google oauth registration intent");
                    (
                        HANDOFF_REGISTRATION_REQUIRED.to_owned(),
                        None,
                        Some(intent.id),
                    )
                }
            }
        },
        _ => return Err(AuthError::BadRequest("OAuth flow неизвестен.".to_owned())),
    };

    let handoff_code = refresh_token::generate();
    state
        .auth_store
        .insert_oauth_handoff(
            refresh_token::hash(&handoff_code),
            kind,
            user_id,
            registration_intent_id,
            now,
            now + Duration::minutes(state.oauth_handoff_lifetime_minutes),
        )
        .await
        .map_err(AuthError::Internal)?;

    Ok(handoff_code)
}

async fn link_google_identity(
    state: &AppState,
    user_id: &Uuid,
    identity: &GoogleIdentity,
    now: chrono::DateTime<Utc>,
) -> Result<(), AuthError> {
    if let Some(account) = state
        .auth_store
        .find_oauth_account_by_subject(GOOGLE_PROVIDER, &identity.subject)
        .await
        .map_err(AuthError::Internal)?
    {
        if account.user_id == *user_id {
            return Ok(());
        }
        return Err(AuthError::Conflict(
            "Этот Google аккаунт уже привязан к другому пользователю.".to_owned(),
        ));
    }
    if state
        .auth_store
        .find_oauth_account_for_user(GOOGLE_PROVIDER, user_id)
        .await
        .map_err(AuthError::Internal)?
        .is_some()
    {
        return Err(AuthError::Conflict(
            "К аккаунту уже привязан Google.".to_owned(),
        ));
    }
    state
        .auth_store
        .insert_oauth_account(
            user_id,
            GOOGLE_PROVIDER.to_owned(),
            identity.subject.clone(),
            identity.email.clone(),
            identity.display_name.clone(),
            now,
        )
        .await
        .map_err(map_oauth_link_error)?;

    Ok(())
}

async fn find_user_or_expired(state: &AppState, user_id: &Uuid) -> Result<UserAccount, AuthError> {
    state
        .auth_store
        .find_user_by_id(user_id)
        .await
        .map_err(AuthError::Internal)?
        .ok_or_else(expired_session)
}

fn linked_account(account: &OAuthAccount) -> LinkedAccount {
    LinkedAccount {
        provider: OAuthProvider::Google,
        email: account.email.clone(),
        display_name: account.display_name.clone(),
        linked_at: account.linked_at.to_rfc3339(),
    }
}

fn map_oauth_link_error(error: anyhow::Error) -> AuthError {
    let message = error.to_string();
    if message.contains("oauth") || message.contains("duplicate") || message.contains("unique") {
        return AuthError::Conflict("Этот Google аккаунт уже привязан.".to_owned());
    }

    AuthError::Internal(error)
}
