//! Проверки внешней ссылки и интервала desktop-входа.

use super::{poll_delay, validate_authorization_url};
use std::time::Duration;
use url::Url;
use uuid::Uuid;

#[test]
fn desktop_sign_in_restricts_poll_delay() {
    assert_eq!(poll_delay(0), Duration::from_secs(2));
    assert_eq!(poll_delay(3), Duration::from_secs(3));
    assert_eq!(poll_delay(u64::MAX), Duration::from_secs(10));
}

#[test]
fn desktop_sign_in_opens_only_expected_confirmation_page() {
    let id = Uuid::parse_str("1234abcd-5678-4123-9123-123456789abc").expect("uuid");
    let origin = Url::parse("https://cheenhub.example").expect("url");
    assert!(
        validate_authorization_url(
            "https://cheenhub.example/auth/desktop?attempt=1234abcd-5678-4123-9123-123456789abc",
            id,
            &origin
        )
        .is_ok()
    );
    let insecure_origin = Url::parse("http://cheenhub.example").expect("origin");
    assert!(
        validate_authorization_url(
            "http://cheenhub.example/auth/desktop?attempt=1234abcd-5678-4123-9123-123456789abc",
            id,
            &insecure_origin,
        )
        .is_err()
    );
    let local_origin = Url::parse("http://127.0.0.1:3000").expect("local");
    assert!(
        validate_authorization_url(
            "http://127.0.0.1:8081/auth/desktop?attempt=1234abcd-5678-4123-9123-123456789abc",
            id,
            &local_origin,
        )
        .is_ok()
    );
    for wrong in [
        "http://cheenhub.example/auth/desktop?attempt=1234abcd-5678-4123-9123-123456789abc",
        "https://cheenhub.example.evil.test/auth/desktop?attempt=1234abcd-5678-4123-9123-123456789abc",
        "https://cheenhub.example@evil.test/auth/desktop?attempt=1234abcd-5678-4123-9123-123456789abc",
        "file:///tmp/desktop",
        "https://cheenhub.example/auth/desktop?attempt=aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        "https://cheenhub.example/auth/desktop?attempt=1234abcd-5678-4123-9123-123456789abc&extra=1",
        "https://cheenhub.example/app?attempt=1234abcd-5678-4123-9123-123456789abc",
    ] {
        assert!(
            validate_authorization_url(wrong, id, &origin).is_err(),
            "accepted {wrong}"
        );
    }
}
