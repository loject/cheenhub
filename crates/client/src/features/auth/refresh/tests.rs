use super::{RefreshError, RefreshFailure, SessionEndReason};
use reqwest::StatusCode;

#[test]
fn transient_refresh_failures_keep_session_retryable() {
    for failure in [
        RefreshFailure::network(),
        RefreshFailure::invalid_response(Some(StatusCode::UNAUTHORIZED)),
        server_failure(StatusCode::TOO_MANY_REQUESTS),
        server_failure(StatusCode::SERVICE_UNAVAILABLE),
        RefreshFailure {
            status: Some(StatusCode::CONFLICT),
            code: Some("refresh_rotation_in_progress".to_owned()),
            message: "concurrent".to_owned(),
            network: false,
        },
    ] {
        assert!(matches!(failure.classify(), RefreshError::Retryable(_)));
    }
}

#[test]
fn confirmed_refresh_rejections_preserve_exact_reason() {
    let invalid = rejected("refresh_token_invalid_or_expired").classify();
    let reused = rejected("refresh_token_reused").classify();

    assert!(matches!(
        invalid,
        RefreshError::SessionEnded {
            reason: SessionEndReason::RefreshTokenInvalidOrExpired,
            ..
        }
    ));
    assert!(matches!(
        reused,
        RefreshError::SessionEnded {
            reason: SessionEndReason::RefreshTokenReused,
            ..
        }
    ));
}

fn server_failure(status: StatusCode) -> RefreshFailure {
    RefreshFailure {
        status: Some(status),
        code: Some("internal_error".to_owned()),
        message: "temporary".to_owned(),
        network: false,
    }
}

fn rejected(code: &str) -> RefreshFailure {
    RefreshFailure {
        status: Some(StatusCode::UNAUTHORIZED),
        code: Some(code.to_owned()),
        message: "rejected".to_owned(),
        network: false,
    }
}
