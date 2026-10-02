//! Подключение, ping-watchdog и backoff realtime-сессии.

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;
use futures_util::future::{Either, FutureExt, select};
use web_time::Instant;

use crate::features::application_focus::application_is_focused;
use crate::features::auth::api as auth_api;
use crate::features::network::{NetworkQualityHandle, realtime as network_realtime};
use crate::features::runtime::sleep_ms;

use super::{RealtimeHandle, RealtimeTransportKind};

const BACKGROUND_PING_INTERVAL_MS: u32 = 2_000;
const VOICE_PING_INTERVAL_MS: u32 = 750;
const PING_ATTEMPT_TIMEOUT_MS: u32 = 1_500;
const VOICE_FAILED_PROBES_BEFORE_RECONNECT: u8 = 1;
const BACKGROUND_FAILED_PROBES_BEFORE_RECONNECT: u8 = 2;
const RECONNECT_INITIAL_DELAY_MS: u32 = 1_000;
const RECONNECT_MAX_DELAY_MS: u32 = 30_000;

/// Поддерживает соединение до отмены владельцем жизненного цикла.
pub(super) async fn run_connection(
    realtime: &RealtimeHandle,
    mut network_quality: NetworkQualityHandle,
    reconnect_requests: &mut mpsc::UnboundedReceiver<()>,
) {
    loop {
        let connection = run_connection_loop(realtime, network_quality).boxed_local();
        let requested = reconnect_requests.next().boxed_local();
        match select(requested, connection).await {
            Either::Left((request, connection)) => {
                drop(connection);
                network_quality.clear();
                realtime.mark_disconnected().await;
                if request.is_none() {
                    return;
                }
                info!("restarting realtime runtime after manual reconnect request");
            }
            Either::Right(((), _)) => return,
        }
    }
}

async fn run_connection_loop(realtime: &RealtimeHandle, mut network_quality: NetworkQualityHandle) {
    let mut reconnect_delay_ms = RECONNECT_INITIAL_DELAY_MS;
    loop {
        realtime.mark_connecting(RealtimeTransportKind::WebTransport);
        info!("opening realtime session");
        let access_token = match auth_api::fresh_access_token().await {
            Ok(access_token) => access_token,
            Err(error) => {
                realtime.mark_disconnected().await;
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
                let mut last_success_at = Instant::now();
                let mut ping_watchdog = PingWatchdog::default();
                let mut probe_wakeup_delay_ms = 0;
                loop {
                    let attempt_started_at = Instant::now();
                    let ping = network_realtime::ping(realtime).boxed_local();
                    let timeout = sleep_ms(PING_ATTEMPT_TIMEOUT_MS).boxed_local();
                    let probe_failed = match select(ping, timeout).await {
                        Either::Left((Ok(measurement), _)) => {
                            network_quality
                                .record_ping(measurement.received_at_ms, measurement.rtt_ms);
                            last_success_at = Instant::now();
                            ping_watchdog.record_success();
                            false
                        }
                        Either::Left((Err(error), _)) => {
                            warn!(
                                %error,
                                application_focused = application_is_focused(),
                                connection_status = ?realtime.connection_status(),
                                attempt_elapsed_ms = attempt_started_at.elapsed().as_millis(),
                                probe_wakeup_delay_ms,
                                elapsed_without_pong_ms = last_success_at.elapsed().as_millis(),
                                "realtime ping attempt failed"
                            );
                            true
                        }
                        Either::Right((_, _)) => {
                            warn!(
                                timeout_ms = PING_ATTEMPT_TIMEOUT_MS,
                                application_focused = application_is_focused(),
                                connection_status = ?realtime.connection_status(),
                                attempt_elapsed_ms = attempt_started_at.elapsed().as_millis(),
                                timer_overrun_ms = attempt_started_at.elapsed().as_millis()
                                    .saturating_sub(u128::from(PING_ATTEMPT_TIMEOUT_MS)),
                                probe_wakeup_delay_ms,
                                elapsed_without_pong_ms =
                                    last_success_at.elapsed().as_millis(),
                                "realtime ping attempt timed out"
                            );
                            true
                        }
                    };
                    let elapsed_without_pong_ms = last_success_at.elapsed().as_millis();
                    let frequent_probes = network_quality.frequent_probes();
                    if probe_failed && ping_watchdog.record_failure(frequent_probes) {
                        let connection_status = realtime.connection_status();
                        network_quality.clear();
                        realtime.mark_disconnected().await;
                        warn!(
                            elapsed_without_pong_ms,
                            ?connection_status,
                            failed_probes = ping_watchdog.consecutive_failed_probes,
                            frequent_probes,
                            delay_ms = reconnect_delay_ms,
                            "realtime ping remained unavailable; reconnecting"
                        );
                        break;
                    }
                    let elapsed_ms = attempt_started_at.elapsed().as_millis();
                    let remaining_ms =
                        remaining_probe_delay_ms(elapsed_ms, network_quality.frequent_probes());
                    if remaining_ms > 0 {
                        let sleep_started_at = Instant::now();
                        sleep_ms(remaining_ms).await;
                        probe_wakeup_delay_ms = sleep_started_at
                            .elapsed()
                            .as_millis()
                            .saturating_sub(u128::from(remaining_ms));
                    } else {
                        probe_wakeup_delay_ms = 0;
                    }
                }
            }
            Err(error) => {
                network_quality.clear();
                realtime.mark_disconnected().await;
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
}

fn next_reconnect_delay(current_ms: u32) -> u32 {
    current_ms.saturating_mul(2).min(RECONNECT_MAX_DELAY_MS)
}

fn probe_interval_ms(frequent_probes: bool) -> u32 {
    if frequent_probes {
        VOICE_PING_INTERVAL_MS
    } else {
        BACKGROUND_PING_INTERVAL_MS
    }
}

fn remaining_probe_delay_ms(attempt_elapsed_ms: u128, frequent_probes: bool) -> u32 {
    u128::from(probe_interval_ms(frequent_probes))
        .saturating_sub(attempt_elapsed_ms)
        .min(u128::from(u32::MAX)) as u32
}

#[derive(Default)]
struct PingWatchdog {
    consecutive_failed_probes: u8,
}

impl PingWatchdog {
    fn record_success(&mut self) {
        self.consecutive_failed_probes = 0;
    }

    fn record_failure(&mut self, frequent_probes: bool) -> bool {
        self.consecutive_failed_probes = self.consecutive_failed_probes.saturating_add(1);
        let failed_probes_before_reconnect = if frequent_probes {
            VOICE_FAILED_PROBES_BEFORE_RECONNECT
        } else {
            BACKGROUND_FAILED_PROBES_BEFORE_RECONNECT
        };
        self.consecutive_failed_probes >= failed_probes_before_reconnect
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PING_ATTEMPT_TIMEOUT_MS, PingWatchdog, probe_interval_ms, remaining_probe_delay_ms,
    };

    #[test]
    fn switches_probe_frequency_only_for_an_active_voice_connection() {
        assert_eq!(probe_interval_ms(false), 2_000);
        assert_eq!(probe_interval_ms(true), 750);
        assert_eq!(PING_ATTEMPT_TIMEOUT_MS, 1_500);
    }

    #[test]
    fn watchdog_uses_one_failed_voice_probe_and_two_failed_background_probes() {
        let mut background = PingWatchdog::default();
        background.record_success();

        let first_background_timeout_at = u64::from(PING_ATTEMPT_TIMEOUT_MS);
        assert_eq!(first_background_timeout_at, 1_500);
        assert!(!background.record_failure(false));

        let second_background_timeout_at = first_background_timeout_at
            + u64::from(remaining_probe_delay_ms(
                u128::from(PING_ATTEMPT_TIMEOUT_MS),
                false,
            ))
            + u64::from(PING_ATTEMPT_TIMEOUT_MS);
        assert_eq!(second_background_timeout_at, 3_500);
        assert!(background.record_failure(false));

        background.record_success();
        assert!(!background.record_failure(false));

        let mut voice = PingWatchdog::default();
        voice.record_success();
        let first_voice_timeout_at = u64::from(PING_ATTEMPT_TIMEOUT_MS);
        assert_eq!(first_voice_timeout_at, 1_500);
        assert!(voice.record_failure(true));
    }
}
