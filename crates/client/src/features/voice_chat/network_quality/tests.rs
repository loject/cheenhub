use std::collections::{HashMap, HashSet};

use super::{
    NetworkQualityFreshness, VoiceNetworkSample, connected_target, freshness_for_age,
    retain_active_samples, sample_is_newer_than_target,
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
fn removes_samples_for_participants_missing_from_the_latest_snapshot() {
    let mut samples = HashMap::from([
        (
            "current".to_owned(),
            VoiceNetworkSample {
                rtt_ms: 20,
                received_at_ms: 1,
            },
        ),
        (
            "active".to_owned(),
            VoiceNetworkSample {
                rtt_ms: 30,
                received_at_ms: 2,
            },
        ),
        (
            "left".to_owned(),
            VoiceNetworkSample {
                rtt_ms: 40,
                received_at_ms: 3,
            },
        ),
    ]);
    let active_user_ids = HashSet::from(["current".to_owned(), "active".to_owned()]);

    retain_active_samples(&mut samples, &active_user_ids);

    assert_eq!(samples.len(), 2);
    assert!(samples.contains_key("current"));
    assert!(samples.contains_key("active"));
    assert!(!samples.contains_key("left"));
}
