use std::collections::{HashMap, HashSet};

use super::{
    NetworkQualityFreshness, VoiceNetworkSample, connected_target, freshness_for_age,
    reading_for_session, retain_active_samples, sample_is_newer_than_target,
};
use crate::features::voice_chat::state::{VoiceConnectionState, VoiceRoomTarget};

#[test]
fn changes_freshness_at_agreed_boundaries() {
    assert_eq!(freshness_for_age(0), NetworkQualityFreshness::Fresh);
    assert_eq!(freshness_for_age(1_499), NetworkQualityFreshness::Fresh);
    assert_eq!(freshness_for_age(1_500), NetworkQualityFreshness::Waiting);
    assert_eq!(freshness_for_age(2_999), NetworkQualityFreshness::Waiting);
    assert_eq!(freshness_for_age(3_000), NetworkQualityFreshness::Unstable);
}

#[test]
fn uses_frequent_probes_only_after_voice_connection_is_established() {
    let target = VoiceRoomTarget::server(
        "server-id".to_string(),
        "room-id".to_string(),
        "Voice room".to_string(),
    );

    assert!(connected_target(&VoiceConnectionState::Disconnected).is_none());
    assert!(
        connected_target(&VoiceConnectionState::Connecting {
            target: target.clone(),
        })
        .is_none()
    );
    assert!(
        connected_target(&VoiceConnectionState::Connected {
            target: target.clone(),
            participants: Vec::new(),
        })
        .is_some()
    );
    assert!(
        connected_target(&VoiceConnectionState::Disconnecting {
            target,
            participants: Vec::new(),
        })
        .is_none()
    );
}

#[test]
fn publishes_only_samples_received_after_target_activation() {
    assert!(!sample_is_newer_than_target(9_999, 10_000));
    assert!(!sample_is_newer_than_target(10_000, 10_000));
    assert!(sample_is_newer_than_target(10_001, 10_000));
}

#[test]
fn removes_samples_for_voice_sessions_missing_from_the_latest_snapshot() {
    let mut samples = HashMap::from([
        (
            ("current".to_owned(), "first".to_owned()),
            VoiceNetworkSample {
                rtt_ms: 20,
                received_at_ms: 1,
            },
        ),
        (
            ("current".to_owned(), "second".to_owned()),
            VoiceNetworkSample {
                rtt_ms: 30,
                received_at_ms: 2,
            },
        ),
        (
            ("left".to_owned(), "third".to_owned()),
            VoiceNetworkSample {
                rtt_ms: 40,
                received_at_ms: 3,
            },
        ),
    ]);
    let active_sessions = HashSet::from([
        ("current".to_owned(), "first".to_owned()),
        ("current".to_owned(), "second".to_owned()),
    ]);

    retain_active_samples(&mut samples, &active_sessions);

    assert_eq!(samples.len(), 2);
    assert!(samples.contains_key(&("current".to_owned(), "first".to_owned())));
    assert!(samples.contains_key(&("current".to_owned(), "second".to_owned())));
    assert!(!samples.contains_key(&("left".to_owned(), "third".to_owned())));
}

#[test]
fn two_sessions_of_one_account_keep_distinct_rtt_readings() {
    let samples = HashMap::from([
        (
            ("same".to_owned(), "device-a".to_owned()),
            VoiceNetworkSample {
                rtt_ms: 12,
                received_at_ms: 1_000,
            },
        ),
        (
            ("same".to_owned(), "device-b".to_owned()),
            VoiceNetworkSample {
                rtt_ms: 130,
                received_at_ms: 1_000,
            },
        ),
    ]);
    assert_eq!(
        reading_for_session(&samples, "same", "device-a", 1_100).rtt_ms,
        Some(12)
    );
    assert_eq!(
        reading_for_session(&samples, "same", "device-b", 1_100).rtt_ms,
        Some(130)
    );
    assert_eq!(
        reading_for_session(&samples, "same", "unknown", 1_100).rtt_ms,
        None
    );
    assert_eq!(
        reading_for_session(&samples, "same", "", 1_100).rtt_ms,
        None
    );
}
