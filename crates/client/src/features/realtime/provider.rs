//! Dioxus-провайдер realtime.

use dioxus::prelude::*;

use crate::features::auth::api as auth_api;
use crate::features::network::{NetworkQualityHandle, realtime as network_realtime};
use crate::features::runtime::sleep_ms;

use super::handle::create_handle;

const PING_INTERVAL_MS: u32 = 5_000;
const RECONNECT_INITIAL_DELAY_MS: u32 = 1_000;
const RECONNECT_MAX_DELAY_MS: u32 = 30_000;

/// Предоставляет realtime-контекст аутентифицированным компонентам приложения.
#[component]
pub(crate) fn RealtimeProvider(children: Element) -> Element {
    let handle = use_signal(create_handle);
    let realtime = handle();
    let context_realtime = realtime.clone();
    use_context_provider(move || context_realtime.clone());
    let network_quality_state = use_signal(Default::default);
    let network_quality = NetworkQualityHandle::new(network_quality_state);
    use_context_provider(move || network_quality);

    use_hook(move || {
        let mut network_quality = network_quality;
        spawn(async move {
            let mut reconnect_delay_ms = RECONNECT_INITIAL_DELAY_MS;
            loop {
                info!("opening realtime session");
                let access_token = match auth_api::fresh_access_token().await {
                    Ok(access_token) => access_token,
                    Err(error) => {
                        warn!(
                            %error,
                            delay_ms = reconnect_delay_ms,
                            "skipping realtime connection without access token"
                        );
                        sleep_ms(reconnect_delay_ms).await;
                        reconnect_delay_ms = next_reconnect_delay(reconnect_delay_ms);
                        continue;
                    }
                };

                match realtime.connect(access_token).await {
                    Ok(authenticated) => {
                        info!(
                            user_id = %authenticated.user.id,
                            "realtime session connected"
                        );
                        reconnect_delay_ms = RECONNECT_INITIAL_DELAY_MS;
                        loop {
                            sleep_ms(PING_INTERVAL_MS).await;
                            match network_realtime::ping(&realtime).await {
                                Ok(measurement) => {
                                    network_quality.record_ping(
                                        measurement.received_at_ms,
                                        measurement.rtt_ms,
                                    );
                                }
                                Err(error) => {
                                    network_quality.clear();
                                    realtime.mark_disconnected().await;
                                    warn!(
                                        %error,
                                        delay_ms = reconnect_delay_ms,
                                        "realtime ping failed; reconnecting"
                                    );
                                    break;
                                }
                            }
                        }
                    }
                    Err(error) => {
                        network_quality.clear();
                        warn!(
                            %error,
                            delay_ms = reconnect_delay_ms,
                            "failed to connect realtime session"
                        );
                    }
                }
                sleep_ms(reconnect_delay_ms).await;
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
