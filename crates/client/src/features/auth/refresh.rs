//! Обновление access token через refresh token.

use cheenhub_contracts::rest::{ApiError, AuthResponse, RefreshRequest};
use dioxus::logger::tracing::{info, warn};
use reqwest::StatusCode;

use crate::features::auth::{jwt, messages, refresh_lock, storage};

const REFRESH_WAIT_ERROR_MESSAGE: &str =
    "Не удалось дождаться обновления сессии в другой вкладке. Попробуй еще раз.";

/// Причина подтверждённого завершения локальной auth-сессии.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionEndReason {
    /// Сохранённые токены отсутствуют, например после выхода в другой вкладке.
    TokensMissing,
    /// Сохранённый access JWT повреждён или имеет неподдерживаемый формат.
    InvalidAccessToken,
    /// Сервер подтвердил, что refresh-токен неизвестен, истёк или относится к завершённой сессии.
    RefreshTokenInvalidOrExpired,
    /// Сервер подтвердил явный отзыв auth-сессии.
    SessionRevoked,
    /// Сервер обнаружил повторное использование refresh-токена и отозвал сессию.
    RefreshTokenReused,
}

/// Данные о завершении auth-сессии, передаваемые из refresh-цикла в UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionEnd {
    /// Точная причина завершения сессии.
    pub(crate) reason: SessionEndReason,
    /// Сообщение, которое можно показать пользователю без повторной классификации.
    pub(crate) message: String,
}

impl SessionEnd {
    pub(super) fn new(reason: SessionEndReason, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }
}

/// Классифицированная ошибка обновления auth-сессии.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RefreshError {
    /// Временная ошибка; сохранённые токены нельзя удалять.
    Retryable(String),
    /// Сервер или локальная проверка подтвердили завершение сессии.
    SessionEnded {
        /// Точная причина завершения сессии.
        reason: SessionEndReason,
        /// Сообщение для журнала и пользовательского уведомления.
        message: String,
    },
}

impl RefreshError {
    /// Возвращает пользовательское описание ошибки.
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Retryable(message) | Self::SessionEnded { message, .. } => message,
        }
    }
}

/// Обновляет access token, сохраняя прежний строковый контракт для обычных API-вызовов.
pub(crate) async fn refresh_access_token() -> Result<String, String> {
    refresh_access_token_classified()
        .await
        .map_err(|error| error.message().to_owned())
}

/// Обновляет access token и возвращает точную классификацию отказа фоновому циклу сессии.
pub(crate) async fn refresh_access_token_classified() -> Result<String, RefreshError> {
    let Some(tokens) = storage::load() else {
        return Err(RefreshError::SessionEnded {
            reason: SessionEndReason::TokensMissing,
            message: "Войди, чтобы продолжить.".to_owned(),
        });
    };

    let _refresh_lock = match refresh_lock::acquire(&tokens).await {
        refresh_lock::RefreshLockOutcome::Acquired(guard) => guard,
        refresh_lock::RefreshLockOutcome::TokensChanged(access_token) => {
            info!("using access token refreshed by another tab");
            return Ok(access_token);
        }
        refresh_lock::RefreshLockOutcome::TimedOut => {
            return Err(RefreshError::Retryable(
                REFRESH_WAIT_ERROR_MESSAGE.to_owned(),
            ));
        }
    };

    if let Some(access_token) = storage::access_token_if_changed(&tokens) {
        info!("using access token refreshed before current tab sent refresh request");
        return Ok(access_token);
    }

    let Some(tokens) = storage::load() else {
        return Err(RefreshError::SessionEnded {
            reason: SessionEndReason::TokensMissing,
            message: "Войди, чтобы продолжить.".to_owned(),
        });
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
            let error = failure.classify();
            if matches!(error, RefreshError::SessionEnded { .. }) {
                storage::clear();
            }
            return Err(error);
        }
    };

    jwt::verify(&response.access_token).map_err(|error| {
        warn!(%error, "auth refresh returned an invalid access token");
        RefreshError::Retryable("Не удалось проверить ответ сервера.".to_owned())
    })?;
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

    fn classify(self) -> RefreshError {
        if self.network
            || self.code.as_deref() == Some("invalid_response")
            || self.status == Some(StatusCode::TOO_MANY_REQUESTS)
            || self.status.is_some_and(|status| status.is_server_error())
        {
            return RefreshError::Retryable(self.message);
        }

        let reason = match self.code.as_deref() {
            Some("refresh_token_reused") => Some(SessionEndReason::RefreshTokenReused),
            Some("refresh_token_invalid_or_expired") => {
                Some(SessionEndReason::RefreshTokenInvalidOrExpired)
            }
            Some("refresh_session_revoked") => Some(SessionEndReason::SessionRevoked),
            Some("unauthorized" | "bad_request")
                if matches!(
                    self.status,
                    Some(StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED)
                ) =>
            {
                Some(SessionEndReason::RefreshTokenInvalidOrExpired)
            }
            _ => None,
        };

        match reason {
            Some(reason) => RefreshError::SessionEnded {
                reason,
                message: self.message,
            },
            None => RefreshError::Retryable(self.message),
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

    if status.is_success() {
        return response
            .json::<AuthResponse>()
            .await
            .map_err(|_| RefreshFailure::invalid_response(Some(status)));
    }

    let error = response
        .json::<ApiError>()
        .await
        .map_err(|_| RefreshFailure::invalid_response(Some(status)))?;
    Err(RefreshFailure {
        status: Some(status),
        code: Some(error.code),
        message: error.message,
        network: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{RefreshError, RefreshFailure, SessionEndReason};
    use reqwest::StatusCode;

    #[test]
    fn transient_refresh_failures_keep_session_retryable() {
        for failure in [
            RefreshFailure::network(),
            RefreshFailure::invalid_response(Some(StatusCode::UNAUTHORIZED)),
            server_failure(StatusCode::TOO_MANY_REQUESTS),
            server_failure(StatusCode::SERVICE_UNAVAILABLE),
            RefreshFailure {
                status: Some(StatusCode::CONFLICT),
                code: Some("refresh_rotation_in_progress".to_owned()),
                message: "concurrent".to_owned(),
                network: false,
            },
        ] {
            assert!(matches!(failure.classify(), RefreshError::Retryable(_)));
        }
    }

    #[test]
    fn confirmed_refresh_rejections_preserve_exact_reason() {
        let invalid = rejected("refresh_token_invalid_or_expired").classify();
        let reused = rejected("refresh_token_reused").classify();

        assert!(matches!(
            invalid,
            RefreshError::SessionEnded {
                reason: SessionEndReason::RefreshTokenInvalidOrExpired,
                ..
            }
        ));
        assert!(matches!(
            reused,
            RefreshError::SessionEnded {
                reason: SessionEndReason::RefreshTokenReused,
                ..
            }
        ));
    }

    fn server_failure(status: StatusCode) -> RefreshFailure {
        RefreshFailure {
            status: Some(status),
            code: Some("internal_error".to_owned()),
            message: "temporary".to_owned(),
            network: false,
        }
    }

    fn rejected(code: &str) -> RefreshFailure {
        RefreshFailure {
            status: Some(StatusCode::UNAUTHORIZED),
            code: Some(code.to_owned()),
            message: "rejected".to_owned(),
            network: false,
        }
    }
}
