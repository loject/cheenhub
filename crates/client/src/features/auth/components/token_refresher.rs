//! Компонент цикла обновления access token.

use dioxus::prelude::*;

use crate::features::auth::{api, jwt, storage};
use crate::features::runtime::sleep_ms;

/// Поддерживает сохраненный access JWT актуальным, пока смонтировано аутентифицированное приложение.
#[component]
pub(crate) fn TokenRefresher(on_session_expired: EventHandler<()>) -> Element {
    let _task = use_hook(move || {
        spawn(async move {
            loop {
                let Some(tokens) = storage::load() else {
                    on_session_expired.call(());
                    break;
                };
                let access_token = tokens.access_token;
                let seconds = match jwt::seconds_until_refresh(&access_token) {
                    Ok(seconds) => seconds,
                    Err(_) => {
                        storage::clear();
                        on_session_expired.call(());
                        break;
                    }
                };

                if seconds > 0 {
                    sleep_ms(seconds.saturating_mul(1000)).await;
                }

                let Some(tokens) = storage::load() else {
                    on_session_expired.call(());
                    break;
                };
                if tokens.access_token != access_token {
                    continue;
                }

                if let Err(error) = api::refresh_access_token().await {
                    if api::is_network_error(&error) {
                        warn!("access token refresh deferred while network is unavailable");
                        sleep_ms(5_000).await;
                        continue;
                    }

                    storage::clear();
                    on_session_expired.call(());
                    break;
                }
            }
        })
    });

    rsx! {}
}
