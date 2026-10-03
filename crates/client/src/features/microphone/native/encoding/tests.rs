use super::*;

#[test]
fn frame_size_matches_twenty_milliseconds() {
    assert_eq!(frame_samples(48_000), 960);
}
