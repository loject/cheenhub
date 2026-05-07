//! Dioxus realtime provider.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::features::auth::api as auth_api;
use crate::features::network::realtime as network_realtime;

use super::handle::create_handle;

const PING_INTERVAL_MS: u32 = 5_000;
const RECONNECT_INITIAL_DELAY_MS: u32 = 1_000;
const RECONNECT_MAX_DELAY_MS: u32 = 30_000;

/// Provides realtime context to authenticated app components.
#[component]
pub(crate) fn RealtimeProvider(children: Element) -> Element {
    let handle = use_signal(create_handle);
    let realtime = handle();
    let context_realtime = realtime.clone();
    use_context_provider(move || context_realtime.clone());

    use_hook(move || {
        spawn(async move {
            let mut reconnect_delay_ms = RECONNECT_INITIAL_DELAY_MS;
            loop {
                info!("opening WebTransport realtime session");
                let access_token = match auth_api::fresh_access_token().await {
                    Ok(access_token) => access_token,
                    Err(error) => {
                        warn!(
                            %error,
                            delay_ms = reconnect_delay_ms,
                            "skipping WebTransport connection without access token"
                        );
                        TimeoutFuture::new(reconnect_delay_ms).await;
                        reconnect_delay_ms = next_reconnect_delay(reconnect_delay_ms);
                        continue;
                    }
                };

                match realtime.connect(access_token).await {
                    Ok(authenticated) => {
                        info!(
                            user_id = %authenticated.user.id,
                            "WebTransport realtime session connected"
                        );
                        reconnect_delay_ms = RECONNECT_INITIAL_DELAY_MS;
                        info!(
                            interval_ms = PING_INTERVAL_MS,
                            "starting WebTransport realtime ping loop"
                        );
                        loop {
                            TimeoutFuture::new(PING_INTERVAL_MS).await;
                            if let Err(error) = network_realtime::ping(&realtime).await {
                                realtime.mark_disconnected().await;
                                warn!(
                                    %error,
                                    delay_ms = reconnect_delay_ms,
                                    "WebTransport realtime ping failed; reconnecting"
                                );
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        warn!(
                            %error,
                            delay_ms = reconnect_delay_ms,
                            "failed to connect WebTransport realtime session"
                        );
                    }
                }
                TimeoutFuture::new(reconnect_delay_ms).await;
                reconnect_delay_ms = next_reconnect_delay(reconnect_delay_ms);
            }
        })
    });

    rsx! {
        {children}
    }
}

fn next_reconnect_delay(current_ms: u32) -> u32 {
    current_ms.saturating_mul(2).min(RECONNECT_MAX_DELAY_MS)
}
