//! Проверки данных страницы передачи сессии в desktop.

use super::{mask_email, verification_code_for_attempt};
use uuid::Uuid;

#[test]
fn browser_confirmation_masks_both_email_parts() {
    assert_eq!(mask_email("ivan@example.com"), "i***@e***.com");
    assert_eq!(mask_email("a@b.org"), "a***@b***.org");
    assert_eq!(mask_email("hello@sub.example.net"), "h***@s***.e***.net");
    assert_eq!(mask_email("invalid"), "***");
    assert_eq!(mask_email("@example.com"), "***");
}

#[test]
fn verification_code_is_stable_and_matches_backend() {
    let id = Uuid::parse_str("1234abcd-5678-4123-9123-123456789abc").expect("uuid");
    assert_eq!(verification_code_for_attempt(id), "1234-ABCD");
}
