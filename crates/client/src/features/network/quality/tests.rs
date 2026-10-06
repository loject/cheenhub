use super::{NetworkQualityState, calculate_jitter, record_sample};

#[test]
fn smooths_rtt_changes_into_jitter() {
    assert_eq!(calculate_jitter(None, None, 100.0), 0.0);
    assert_eq!(calculate_jitter(Some(100.0), None, 140.0), 40.0);
    assert_eq!(calculate_jitter(Some(140.0), Some(40.0), 100.0), 40.0);
}

#[test]
fn keeps_only_the_latest_minute_at_voice_ping_frequency() {
    let mut state = NetworkQualityState::default();

    for index in 0..=100 {
        record_sample(&mut state, index * 750, 20.0 + index as f64);
    }

    assert_eq!(state.samples.len(), 81);
    assert_eq!(
        state.samples.first().map(|sample| sample.received_at_ms),
        Some(15_000)
    );
    assert_eq!(
        state.samples.last().map(|sample| sample.received_at_ms),
        Some(75_000)
    );
}
