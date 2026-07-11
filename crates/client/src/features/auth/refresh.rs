//! Обновление access token через refresh token.

use cheenhub_contracts::rest::{ApiError, AuthResponse, RefreshRequest};
use dioxus::logger::tracing::{info, warn};
use reqwest::StatusCode;

use crate::features::auth::{jwt, messages, refresh_lock, storage};

const REFRESH_WAIT_ERROR_MESSAGE: &str =
    "Не удалось дождаться обновления сессии в другой вкладке. Попробуй еще раз.";

/// Возвращает, можно ли отложить refresh без принудительного выхода.
pub(crate) fn is_retryable_refresh_error(error: &str) -> bool {
    error == messages::NETWORK_ERROR_MESSAGE || error == REFRESH_WAIT_ERROR_MESSAGE
}

/// Обновляет access token, координируя одноразовый refresh token между вкладками.
pub(crate) async fn refresh_access_token() -> Result<String, String> {
    let Some(tokens) = storage::load() else {
        return Err("Войди, чтобы продолжить.".to_owned());
    };

    let _refresh_lock = match refresh_lock::acquire(&tokens).await {
        refresh_lock::RefreshLockOutcome::Acquired(guard) => guard,
        refresh_lock::RefreshLockOutcome::TokensChanged(access_token) => {
            info!("using access token refreshed by another tab");
            return Ok(access_token);
        }
        refresh_lock::RefreshLockOutcome::TimedOut => {
            return Err(REFRESH_WAIT_ERROR_MESSAGE.to_owned());
        }
    };

    if let Some(access_token) = storage::access_token_if_changed(&tokens) {
        info!("using access token refreshed before current tab sent refresh request");
        return Ok(access_token);
    }

    let Some(tokens) = storage::load() else {
        return Err("Войди, чтобы продолжить.".to_owned());
    };
    let response: AuthResponse = match post_refresh_json(&tokens.refresh_token).await {
        Ok(response) => response,
        Err(failure) => {
            warn!(
                status = ?failure.status,
                code = failure.code.as_deref().unwrap_or("unknown"),
                network = failure.network,
                "auth refresh request failed"
            );
            if let Some(access_token) = storage::access_token_if_changed(&tokens) {
                info!("using access token refreshed by another tab after refresh error");
                return Ok(access_token);
            }
            if !failure.network {
                storage::clear();
            }
            return Err(failure.message);
        }
    };

    jwt::verify(&response.access_token)?;
    storage::save(&response.access_token, &response.refresh_token);
    info!("refreshed auth tokens");
    Ok(response.access_token)
}

#[derive(Debug)]
struct RefreshFailure {
    status: Option<StatusCode>,
    code: Option<String>,
    message: String,
    network: bool,
}

impl RefreshFailure {
    fn network() -> Self {
        Self {
            status: None,
            code: None,
            message: messages::NETWORK_ERROR_MESSAGE.to_owned(),
            network: true,
        }
    }

    fn invalid_response(status: Option<StatusCode>) -> Self {
        Self {
            status,
            code: Some("invalid_response".to_owned()),
            message: "Не удалось прочитать ответ сервера.".to_owned(),
            network: false,
        }
    }
}

async fn post_refresh_json(refresh_token: &str) -> Result<AuthResponse, RefreshFailure> {
    let response = super::http::post("/auth/refresh")
        .json(&RefreshRequest {
            refresh_token: refresh_token.to_owned(),
        })
        .send()
        .await
        .map_err(|_| RefreshFailure::network())?;
    let status = response.status();

    if response.status().is_success() {
        return response
            .json::<AuthResponse>()
            .await
            .map_err(|_| RefreshFailure::invalid_response(Some(status)));
    }

    let error = response.json::<ApiError>().await.ok();
    Err(RefreshFailure {
        status: Some(status),
        code: error.as_ref().map(|error| error.code.clone()),
        message: error
            .map(|error| error.message)
            .unwrap_or_else(|| "Не удалось выполнить запрос. Попробуй еще раз.".to_owned()),
        network: false,
    })
}
