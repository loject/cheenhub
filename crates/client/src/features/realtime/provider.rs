//! Dioxus realtime provider.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::features::auth::api as auth_api;

use super::handle::create_handle;

const PING_INTERVAL_MS: u32 = 5_000;

/// Provides realtime context to authenticated app components.
#[component]
pub(crate) fn RealtimeProvider(children: Element) -> Element {
    let handle = use_signal(create_handle);
    let realtime = handle();
    let context_realtime = realtime.clone();
    use_context_provider(move || context_realtime.clone());

    use_hook(move || {
        spawn(async move {
            info!("opening WebTransport realtime session");
            let access_token = match auth_api::fresh_access_token().await {
                Ok(access_token) => access_token,
                Err(error) => {
                    warn!(%error, "skipping WebTransport connection without access token");
                    return;
                }
            };

            match realtime.connect(access_token).await {
                Ok(authenticated) => {
                    info!(
                        user_id = %authenticated.user.id,
                        "WebTransport realtime session connected"
                    );
                    info!(
                        interval_ms = PING_INTERVAL_MS,
                        "starting WebTransport realtime ping loop"
                    );
                    loop {
                        if let Err(error) = realtime.ping().await {
                            warn!(%error, "stopping WebTransport realtime ping loop");
                            break;
                        }
                        TimeoutFuture::new(PING_INTERVAL_MS).await;
                    }
                }
                Err(error) => {
                    warn!(%error, "failed to connect WebTransport realtime session");
                }
            }
        })
    });

    rsx! {
        {children}
    }
}
