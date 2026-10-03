use super::parse_jitter_buffer_us;

#[test]
fn reads_legacy_integer_milliseconds_as_microseconds() {
    assert_eq!(parse_jitter_buffer_us("120"), Some(120_000));
}

#[test]
fn reads_fractional_milliseconds_as_microseconds() {
    assert_eq!(parse_jitter_buffer_us("0.5"), Some(500));
}
