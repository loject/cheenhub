use super::{friendly_message_date, full_message_datetime, message_day_key};

#[test]
fn formats_full_datetime_for_tooltip() {
    assert_eq!(
        full_message_datetime("2025-07-12T08:09:10+00:00"),
        "12 июля 2025 г., 08:09:10"
    );
}

#[test]
fn uses_full_date_for_messages_older_than_year() {
    assert_eq!(
        friendly_message_date("2020-07-12T08:09:10+00:00"),
        "12 июля 2020"
    );
}

#[test]
fn groups_equivalent_timestamps_by_utc_day() {
    assert_eq!(
        message_day_key("2025-07-12T23:30:00-02:00"),
        message_day_key("2025-07-13T01:30:00+00:00")
    );
}
