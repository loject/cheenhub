//! Компонент цикла обновления access token.

use dioxus::prelude::*;

use crate::features::auth::refresh::{
    AccessTokenRecovery, RefreshError, SessionEnd, SessionEndReason, recover_invalid_access_token,
};
use crate::features::auth::{jwt, storage};
use crate::features::runtime::sleep_ms;

/// Поддерживает сохраненный access JWT актуальным, пока смонтировано аутентифицированное приложение.
#[component]
pub(crate) fn TokenRefresher(on_session_expired: EventHandler<SessionEnd>) -> Element {
    let _task = use_hook(move || {
        spawn(async move {
            loop {
                let Some(tokens) = storage::load() else {
                    on_session_expired.call(SessionEnd::new(
                        SessionEndReason::TokensMissing,
                        "Сессия завершена в другой вкладке или окне.",
                    ));
                    break;
                };
                let access_token = tokens.access_token;
                let seconds = match jwt::seconds_until_refresh(&access_token) {
                    Ok(seconds) => seconds,
                    Err(error) => {
                        warn!(%error, "stored access token is invalid; trying to recover it with the refresh token");
                        match recover_invalid_access_token().await {
                            AccessTokenRecovery::Recovered => {
                                info!("replaced an invalid access token using the refresh token");
                                continue;
                            }
                            AccessTokenRecovery::RetryLater(message) => {
                                warn!(%message, "access token recovery deferred");
                                sleep_ms(5_000).await;
                                continue;
                            }
                            AccessTokenRecovery::SessionEnded(session_end) => {
                                warn!(
                                    reason = ?session_end.reason,
                                    %session_end.message,
                                    "access token recovery confirmed the session is no longer valid"
                                );
                                on_session_expired.call(session_end);
                                break;
                            }
                        }
                    }
                };

                if seconds > 0 {
                    sleep_ms(seconds.saturating_mul(1000)).await;
                }

                let Some(tokens) = storage::load() else {
                    on_session_expired.call(SessionEnd::new(
                        SessionEndReason::TokensMissing,
                        "Сессия завершена в другой вкладке или окне.",
                    ));
                    break;
                };
                if tokens.access_token != access_token {
                    continue;
                }

                if let Err(error) = super::super::refresh::refresh_access_token_classified().await {
                    match error {
                        RefreshError::Retryable(message) => {
                            warn!(%message, "access token refresh deferred");
                            sleep_ms(5_000).await;
                        }
                        RefreshError::SessionEnded { reason, message } => {
                            warn!(?reason, %message, "access token refresh ended client session");
                            on_session_expired.call(SessionEnd::new(reason, message));
                            break;
                        }
                    }
                }
            }
        })
    });

    rsx! {}
}
