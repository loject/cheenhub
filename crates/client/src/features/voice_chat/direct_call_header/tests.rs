use super::format_duration;

#[test]
fn formats_short_and_long_call_durations() {
    assert_eq!(format_duration(62), "01:02");
    assert_eq!(format_duration(3_661), "1:01:01");
}
