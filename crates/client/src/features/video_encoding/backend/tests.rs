use super::VideoFrameRateGate;

#[test]
fn frame_rate_gate_limits_30_fps_source_to_15_fps() {
    let mut gate = VideoFrameRateGate::new(15);
    let timestamps = [0, 33_333, 66_666, 99_999, 133_332];
    let accepted = timestamps
        .into_iter()
        .filter(|timestamp| gate.accept(*timestamp))
        .collect::<Vec<_>>();

    assert_eq!(accepted, vec![0, 66_666, 133_332]);
}

#[test]
fn frame_rate_gate_recovers_after_timestamp_reset() {
    let mut gate = VideoFrameRateGate::new(24);

    assert!(gate.accept(100_000));
    assert!(gate.accept(1_000));
}
