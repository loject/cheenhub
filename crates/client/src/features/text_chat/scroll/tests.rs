use super::*;

#[test]
fn older_history_loads_only_at_the_top_in_an_idle_ready_list() {
    assert!(should_load_older(
        OLDER_PAGE_SCROLL_THRESHOLD,
        true,
        false,
        false
    ));
    assert!(!should_load_older(
        OLDER_PAGE_SCROLL_THRESHOLD + 0.1,
        true,
        false,
        false
    ));
    assert!(!should_load_older(0.0, false, false, false));
    assert!(!should_load_older(0.0, true, true, false));
    assert!(!should_load_older(0.0, true, false, true));
}

#[test]
fn bottom_detection_includes_the_existing_threshold() {
    assert!(is_offset_near_bottom(476.0, 1_000.0, 500.0));
    assert!(!is_offset_near_bottom(475.9, 1_000.0, 500.0));
    assert!(is_offset_near_bottom(0.0, 300.0, 500.0));
}

#[test]
fn preserving_scroll_adds_content_growth_above_the_viewport() {
    assert_eq!(preserved_scroll_offset(20.0, 1_000.0, 1_450.0), 470.0);
}

#[test]
fn preserving_scroll_never_returns_a_negative_offset() {
    assert_eq!(preserved_scroll_offset(20.0, 1_000.0, 500.0), 0.0);
}
