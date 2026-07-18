//! Компонент цикла обновления access token.

use dioxus::prelude::*;

use crate::features::auth::refresh::{RefreshError, SessionEnd, SessionEndReason};
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
                        warn!(%error, "stored access token is invalid; ending client session");
                        storage::clear();
                        on_session_expired.call(SessionEnd::new(
                            SessionEndReason::InvalidAccessToken,
                            "Данные сессии повреждены. Войди снова.",
                        ));
                        break;
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
