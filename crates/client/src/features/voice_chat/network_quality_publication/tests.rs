use super::NetworkQualityPublicationState;

#[test]
fn publishes_first_sample_significant_changes_and_quality_transitions() {
    let mut state = NetworkQualityPublicationState::default();
    assert!(state.should_publish(100, 10_000));
    state.mark_published(100, 10_000);

    assert!(!state.should_publish(110, 10_750));
    assert!(state.should_publish(125, 10_750));

    state.mark_published(150, 10_000);
    assert!(state.should_publish(151, 10_750));
}

#[test]
fn stable_rtt_uses_a_periodic_heartbeat_without_extra_fanout() {
    let mut state = NetworkQualityPublicationState::default();
    state.mark_published(50, 10_000);

    assert!(!state.should_publish(55, 10_750));
    assert!(!state.should_publish(55, 10_999));
    assert!(state.should_publish(55, 11_000));
}

#[test]
fn failed_publication_can_retry_the_same_rtt_before_heartbeat() {
    let mut state = NetworkQualityPublicationState::default();

    let result = state.finish_attempt(50, 10_000, Err::<(), _>("offline"));

    assert_eq!(result, Err("offline"));
    assert!(state.should_publish(50, 10_250));
}
