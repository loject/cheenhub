use super::measured_overflow;

#[test]
fn date_bubbles_require_content_outside_the_viewport() {
    assert_eq!(measured_overflow(300.0, 600.0), Some(false));
    assert_eq!(measured_overflow(600.0, 600.0), Some(false));
    assert_eq!(measured_overflow(600.5, 600.0), Some(false));
    assert_eq!(measured_overflow(602.0, 600.0), Some(true));
}

#[test]
fn resizing_back_to_fit_restores_date_lines() {
    assert_eq!(measured_overflow(900.0, 600.0), Some(true));
    assert_eq!(measured_overflow(900.0, 1000.0), Some(false));
}

#[test]
fn hidden_or_invalid_viewports_do_not_replace_the_current_mode() {
    assert_eq!(measured_overflow(900.0, 0.0), None);
    assert_eq!(measured_overflow(900.0, -1.0), None);
    assert_eq!(measured_overflow(f64::NAN, 600.0), None);
    assert_eq!(measured_overflow(900.0, f64::INFINITY), None);
}
