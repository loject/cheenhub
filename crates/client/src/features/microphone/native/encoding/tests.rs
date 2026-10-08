use super::*;

#[test]
fn frame_size_matches_ten_milliseconds_at_supported_rates() {
    assert_eq!(frame_samples(48_000), 480);
    assert_eq!(frame_samples(24_000), 240);
    assert_eq!(frame_samples(16_000), 160);
}
