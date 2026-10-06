use super::{PING_ATTEMPT_TIMEOUT_MS, PingWatchdog, probe_interval_ms, remaining_probe_delay_ms};

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
