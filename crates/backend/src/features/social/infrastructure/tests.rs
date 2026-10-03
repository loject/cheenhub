//! Проверки инфраструктуры социальных данных.

use super::{normalize_unread_count, unread_count_after_read};

#[test]
fn unread_count_after_read_never_goes_below_zero() {
    assert_eq!(unread_count_after_read(0, 77), 0);
    assert_eq!(unread_count_after_read(-77, 1), 0);
    assert_eq!(unread_count_after_read(5, 2), 3);
}

#[test]
fn normalize_unread_count_repairs_legacy_negative_values() {
    assert_eq!(normalize_unread_count(-76), 0);
    assert_eq!(normalize_unread_count(3), 3);
}
