//! Access token refresh loop component.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::features::auth::{api, jwt, storage};

/// Keeps the stored access JWT fresh while the authenticated app is mounted.
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
                    TimeoutFuture::new(seconds.saturating_mul(1000)).await;
                }

                let Some(tokens) = storage::load() else {
                    on_session_expired.call(());
                    break;
                };
                if tokens.access_token != access_token {
                    continue;
                }

                if api::refresh_access_token().await.is_err() {
                    storage::clear();
                    on_session_expired.call(());
                    break;
                }
            }
        })
    });

    rsx! {}
}
