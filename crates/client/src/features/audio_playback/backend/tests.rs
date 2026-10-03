use super::target_playout_depth_seconds;

#[test]
fn target_playout_depth_follows_jitter_buffer_setting() {
    assert!((target_playout_depth_seconds(10_000) - 0.01).abs() < 1e-9);
    assert!((target_playout_depth_seconds(200_000) - 0.2).abs() < 1e-9);
}

#[test]
fn target_playout_depth_supports_small_settings() {
    assert!((target_playout_depth_seconds(500) - 0.0005).abs() < 1e-9);
    assert_eq!(target_playout_depth_seconds(0), 0.0);
}
