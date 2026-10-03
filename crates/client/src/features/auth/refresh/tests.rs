use super::{
    AccessTokenRecovery, RefreshError, RefreshFailure, SessionEnd, SessionEndReason,
    classify_recovery,
};
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

#[test]
fn successful_refresh_recovers_an_invalid_access_token() {
    assert_eq!(
        classify_recovery(Ok("new-access-token".to_owned())),
        AccessTokenRecovery::Recovered
    );
}

#[test]
fn transient_refresh_failure_defers_access_token_recovery() {
    let recovery = classify_recovery(Err(RefreshError::Retryable("offline".to_owned())));

    assert_eq!(
        recovery,
        AccessTokenRecovery::RetryLater("offline".to_owned())
    );
}

#[test]
fn confirmed_refresh_rejection_ends_session_during_access_token_recovery() {
    let recovery = classify_recovery(Err(RefreshError::SessionEnded {
        reason: SessionEndReason::RefreshTokenInvalidOrExpired,
        message: "expired".to_owned(),
    }));

    assert_eq!(
        recovery,
        AccessTokenRecovery::SessionEnded(SessionEnd::new(
            SessionEndReason::RefreshTokenInvalidOrExpired,
            "expired"
        ))
    );
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
