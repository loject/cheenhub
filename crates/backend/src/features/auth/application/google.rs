//! Google OAuth provider integration helpers.

use anyhow::Context;
use serde::Deserialize;
use url::Url;

use crate::features::auth::error::AuthError;
use crate::features::auth::validation;
use crate::state::AppState;

/// Google OAuth client configuration.
#[derive(Debug, Clone)]
pub(super) struct GoogleConfig {
    /// Google OAuth client id.
    pub(super) client_id: String,
    /// Google OAuth client secret.
    pub(super) client_secret: String,
    /// Registered backend callback URL.
    pub(super) redirect_uri: String,
}

/// Verified Google identity.
#[derive(Debug, Clone)]
pub(super) struct GoogleIdentity {
    /// Stable Google subject.
    pub(super) subject: String,
    /// Verified Google email.
    pub(super) email: String,
    /// Google display name.
    pub(super) display_name: Option<String>,
}

/// Loads Google OAuth configuration from application state.
pub(super) fn google_config(state: &AppState) -> Result<GoogleConfig, AuthError> {
    let mut missing = Vec::new();
    if state.google_oauth_client_id.is_none() {
        missing.push("GOOGLE_OAUTH_CLIENT_ID");
    }
    if state.google_oauth_client_secret.is_none() {
        missing.push("GOOGLE_OAUTH_CLIENT_SECRET");
    }
    if state.google_oauth_redirect_uri.is_none() {
        missing.push("GOOGLE_OAUTH_REDIRECT_URI");
    }
    if !missing.is_empty() {
        return Err(AuthError::Misconfigured {
            feature: "google_oauth",
            missing,
            message: "Вход через Google не настроен на сервере.".to_owned(),
        });
    }

    Ok(GoogleConfig {
        client_id: state
            .google_oauth_client_id
            .clone()
            .expect("google oauth client id was checked"),
        client_secret: state
            .google_oauth_client_secret
            .clone()
            .expect("google oauth client secret was checked"),
        redirect_uri: state
            .google_oauth_redirect_uri
            .clone()
            .expect("google oauth redirect uri was checked"),
    })
}

/// Builds the frontend OAuth callback URL.
pub(super) fn frontend_oauth_url(state: &AppState, params: &[(&str, &str)]) -> String {
    let base = format!(
        "{}/auth/oauth/google",
        state.cheenhub_client_base_url.trim_end_matches('/')
    );
    let mut url: Url = match Url::parse(&base) {
        Ok(url) => url,
        Err(_) => return base,
    };
    for (key, value) in params {
        url.query_pairs_mut().append_pair(key, value);
    }

    url.to_string()
}

/// Exchanges an authorization code and verifies the returned Google identity.
pub(super) async fn exchange_google_code(
    config: &GoogleConfig,
    code: &str,
    expected_nonce: &str,
) -> Result<GoogleIdentity, AuthError> {
    let client = reqwest::Client::new();
    let token = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("redirect_uri", config.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .context("failed to exchange google oauth code")
        .map_err(AuthError::Internal)?;
    if !token.status().is_success() {
        return Err(AuthError::Unauthorized(
            "Google не подтвердил вход. Попробуй еще раз.".to_owned(),
        ));
    }
    let token = token
        .json::<GoogleTokenResponse>()
        .await
        .context("failed to decode google oauth token response")
        .map_err(AuthError::Internal)?;
    let token_info_url = Url::parse_with_params(
        "https://oauth2.googleapis.com/tokeninfo",
        &[("id_token", token.id_token.as_str())],
    )
    .map_err(anyhow::Error::from)?;
    let token_info = client
        .get(token_info_url)
        .send()
        .await
        .context("failed to verify google id token")
        .map_err(AuthError::Internal)?;
    if !token_info.status().is_success() {
        return Err(AuthError::Unauthorized(
            "Google не подтвердил личность. Попробуй еще раз.".to_owned(),
        ));
    }
    let token_info = token_info
        .json::<GoogleTokenInfo>()
        .await
        .context("failed to decode google id token info")
        .map_err(AuthError::Internal)?;

    if token_info.aud != config.client_id
        || !matches!(
            token_info.iss.as_str(),
            "https://accounts.google.com" | "accounts.google.com"
        )
        || token_info.nonce.as_deref() != Some(expected_nonce)
        || token_info.email_verified != "true"
    {
        return Err(AuthError::Unauthorized(
            "Google не подтвердил email аккаунта.".to_owned(),
        ));
    }
    let email = token_info
        .email
        .filter(|email| validation::is_valid_email(&email.to_lowercase()))
        .ok_or_else(|| AuthError::Unauthorized("Google не вернул корректный email.".to_owned()))?;

    Ok(GoogleIdentity {
        subject: token_info.sub,
        email,
        display_name: token_info.name,
    })
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    id_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenInfo {
    aud: String,
    iss: String,
    sub: String,
    email: Option<String>,
    email_verified: String,
    nonce: Option<String>,
    name: Option<String>,
}
