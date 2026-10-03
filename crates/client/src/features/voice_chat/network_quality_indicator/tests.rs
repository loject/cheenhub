use super::{IndicatorDisclosureState, NetworkQualityTone, format_rtt, reading_tone, status_label};
use crate::features::voice_chat::network_quality::{
    NetworkQualityFreshness, VoiceNetworkQualityReading,
};

#[test]
fn formats_live_and_stale_network_readings() {
    let fresh = VoiceNetworkQualityReading {
        rtt_ms: Some(9),
        freshness: NetworkQualityFreshness::Fresh,
    };
    let waiting = VoiceNetworkQualityReading {
        rtt_ms: Some(42),
        freshness: NetworkQualityFreshness::Waiting,
    };
    let unstable = VoiceNetworkQualityReading {
        rtt_ms: None,
        freshness: NetworkQualityFreshness::Unstable,
    };

    assert_eq!(format_rtt(fresh), "9 мс");
    assert_eq!(format_rtt(waiting), "42 мс");
    assert_eq!(
        status_label(waiting, reading_tone(waiting)),
        Some("Нет свежих данных")
    );
    assert_eq!(
        status_label(unstable, reading_tone(unstable)),
        Some("Соединение нестабильно")
    );
}

#[test]
fn missing_first_rtt_is_neutral_while_measuring() {
    let measuring = VoiceNetworkQualityReading {
        rtt_ms: None,
        freshness: NetworkQualityFreshness::Measuring,
    };

    assert_eq!(reading_tone(measuring), NetworkQualityTone::Neutral);
    assert_eq!(
        status_label(measuring, reading_tone(measuring)),
        Some("Измеряем…")
    );
}

#[test]
fn classifies_fresh_rtt_at_voice_quality_boundaries() {
    let reading = |rtt_ms| VoiceNetworkQualityReading {
        rtt_ms: Some(rtt_ms),
        freshness: NetworkQualityFreshness::Fresh,
    };

    assert_eq!(reading_tone(reading(150)), NetworkQualityTone::Good);
    assert_eq!(reading_tone(reading(151)), NetworkQualityTone::Degraded);
    assert_eq!(reading_tone(reading(500)), NetworkQualityTone::Degraded);
    assert_eq!(reading_tone(reading(501)), NetworkQualityTone::Poor);
    assert_eq!(reading_tone(reading(800)), NetworkQualityTone::Poor);
    assert_eq!(reading_tone(reading(1_400)), NetworkQualityTone::Poor);
}

#[test]
fn freshness_can_only_worsen_the_rtt_tone() {
    let reading = |freshness| VoiceNetworkQualityReading {
        rtt_ms: Some(20),
        freshness,
    };

    assert_eq!(
        reading_tone(reading(NetworkQualityFreshness::Waiting)),
        NetworkQualityTone::Degraded
    );
    assert_eq!(
        reading_tone(reading(NetworkQualityFreshness::Unstable)),
        NetworkQualityTone::Poor
    );
}

#[test]
fn hover_is_temporary_and_click_pins_the_indicator() {
    let mut state = IndicatorDisclosureState::default();
    assert!(!state.expanded());

    state.set_mouse_hovered(true);
    assert!(state.expanded());
    state.set_mouse_hovered(false);
    assert!(!state.expanded());

    state.toggle_pinned();
    assert!(state.expanded());
    state.set_mouse_hovered(true);
    state.set_mouse_hovered(false);
    assert!(state.expanded());
    state.toggle_pinned();
    assert!(!state.expanded());

    state.toggle_pinned();
    assert!(state.expanded());
    state.dismiss();
    assert!(!state.expanded());
}
