//! Тесты расчёта TTL кеша JWKS Google.

use super::*;
use reqwest::header::HeaderValue;

#[test]
fn jwks_cache_ttl_uses_bounded_cache_control_max_age() {
    assert_eq!(
        cache_ttl(Some(&HeaderValue::from_static("public, max-age=3600"))),
        StdDuration::from_secs(3600)
    );
    assert_eq!(
        cache_ttl(Some(&HeaderValue::from_static("max-age=1"))),
        GOOGLE_JWKS_MIN_TTL
    );
    assert_eq!(
        cache_ttl(Some(&HeaderValue::from_static("max-age=999999"))),
        GOOGLE_JWKS_MAX_TTL
    );
}
