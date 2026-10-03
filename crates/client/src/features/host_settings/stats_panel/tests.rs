use super::{format_count, last_minute_value, peak_minute_value};
use cheenhub_contracts::rest::HostMessagesPerMinuteSample;

fn sample(minute_offset: i64, messages: u64) -> HostMessagesPerMinuteSample {
    HostMessagesPerMinuteSample {
        minute_unix_ms: minute_offset * 60_000,
        messages,
    }
}

#[test]
fn small_counts_are_shown_without_thousand_separators() {
    assert_eq!(format_count(0), "0");
    assert_eq!(format_count(999), "999");
    assert_eq!(format_count(1_000), "1000");
    assert_eq!(format_count(9_999), "9999");
    assert_eq!(format_count(10_000), "10 000");
    assert_eq!(format_count(12_345), "12 345");
    assert_eq!(format_count(1_234_567), "1 234 567");
}

#[test]
fn last_and_peak_minute_values_come_from_samples() {
    let samples = vec![sample(0, 3), sample(1, 0), sample(2, 9)];

    assert_eq!(last_minute_value(&samples), 9);
    assert_eq!(peak_minute_value(&samples), 9);
    assert_eq!(last_minute_value(&[]), 0);
    assert_eq!(peak_minute_value(&[]), 0);
}
