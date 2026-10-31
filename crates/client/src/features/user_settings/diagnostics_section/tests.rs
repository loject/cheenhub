use super::*;

#[test]
fn datetime_filter_converts_local_time_to_rfc3339() {
    let bound = datetime_bound("2026-10-31T14:20").unwrap().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(&bound).is_ok());
    assert_eq!(datetime_bound("").unwrap(), None);
    assert!(datetime_bound("not-a-date").is_err());
}

#[test]
fn upper_bound_includes_entire_selected_minute() {
    let filter = filter_from_input("", "", "", "", "2026-10-31T14:20").unwrap();
    let end = chrono::DateTime::parse_from_rfc3339(filter.until.as_deref().unwrap()).unwrap();
    assert_eq!(
        end.with_timezone(&Local).format("%H:%M:%S%.9f").to_string(),
        "14:20:59.999999999"
    );
}

#[test]
fn timestamp_display_uses_local_zone() {
    let source = "2026-10-31T10:00:00Z";
    let local = chrono::DateTime::parse_from_rfc3339(source)
        .unwrap()
        .with_timezone(&Local);
    assert_eq!(
        format_local_timestamp(source),
        local.format("%Y-%m-%d %H:%M:%S %:z").to_string()
    );
    assert_eq!(format_local_timestamp("invalid"), "invalid");
}

#[test]
fn time_filter_rejects_inverted_interval() {
    let error = filter_from_input(
        "WARN",
        "voice",
        "late",
        "2026-10-31T15:00",
        "2026-10-31T14:00",
    );
    assert!(error.is_err());
    let valid = filter_from_input("WARN", " voice ", " late ", "", "").unwrap();
    assert_eq!(valid.level.as_deref(), Some("WARN"));
    assert_eq!(valid.target, "voice");
    assert_eq!(valid.text, "late");
}
