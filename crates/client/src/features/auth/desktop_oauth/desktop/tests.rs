use super::{finish, poll_delay, validate_authorization_url};
use cheenhub_contracts::rest::OAuthFlow;
use std::time::Duration;
#[test]
fn bounds_polling_interval() {
    assert_eq!(poll_delay(0), Duration::from_secs(2));
    assert_eq!(poll_delay(3), Duration::from_secs(3));
    assert_eq!(poll_delay(u64::MAX), Duration::from_secs(10));
}
#[test]
fn opens_only_google_https_authorization_urls() {
    assert!(
        validate_authorization_url("https://accounts.google.com/o/oauth2/v2/auth?state=example")
            .is_ok()
    );
    for invalid in [
        "file:///tmp/auth",
        "http://accounts.google.com/auth",
        "https://evil.test/auth",
        "https://accounts.google.com@evil.test/auth",
    ] {
        assert!(validate_authorization_url(invalid).is_err());
    }
}
#[test]
fn linking_rejects_session_and_registration_payloads() {
    assert!(finish(serde_json::json!({"kind":"linked"}), OAuthFlow::Link).is_ok());
    assert!(finish(serde_json::json!({"auth_response": {}}), OAuthFlow::Link).is_err());
    assert!(
        finish(
            serde_json::json!({"kind":"registration_required"}),
            OAuthFlow::Link
        )
        .is_err()
    );
}
