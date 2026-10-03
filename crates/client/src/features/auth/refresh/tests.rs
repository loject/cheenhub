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

#[test]
fn locally_expired_refresh_response_preserves_rotated_tokens_for_delayed_retry() {
    use crate::features::auth::{jwt::JwtVerifyError, storage::StoredTokens};
    use std::cell::RefCell;

    let stored = RefCell::new(Some(StoredTokens {
        access_token: "old-access".to_owned(),
        refresh_token: "consumed-refresh".to_owned(),
    }));

    let result = super::apply_refresh_tokens(
        "signed-but-locally-expired-access",
        "rotated-refresh",
        Err(JwtVerifyError::Expired),
        |access_token, refresh_token| {
            *stored.borrow_mut() = Some(StoredTokens {
                access_token: access_token.to_owned(),
                refresh_token: refresh_token.to_owned(),
            });
        },
        || {
            *stored.borrow_mut() = None;
        },
    );

    assert_eq!(
        stored.into_inner(),
        Some(StoredTokens {
            access_token: "signed-but-locally-expired-access".to_owned(),
            refresh_token: "rotated-refresh".to_owned(),
        })
    );
    assert!(matches!(
        classify_recovery(result),
        AccessTokenRecovery::RetryLater(_)
    ));
}

#[test]
fn fresh_refresh_response_saves_rotated_pair_and_returns_access_token() {
    let mut stored = None;

    let result = super::apply_refresh_tokens(
        "fresh-access",
        "rotated-refresh",
        Ok(()),
        |access_token, refresh_token| {
            stored = Some((access_token.to_owned(), refresh_token.to_owned()));
        },
        || panic!("fresh tokens must not clear the session"),
    );

    assert_eq!(result, Ok("fresh-access".to_owned()));
    assert_eq!(
        stored,
        Some(("fresh-access".to_owned(), "rotated-refresh".to_owned()))
    );
}

#[test]
fn unauthentic_refresh_response_clears_session_without_saving_tokens() {
    use crate::features::auth::jwt::JwtVerifyError;

    for error in [
        JwtVerifyError::InvalidToken,
        JwtVerifyError::VerificationKeyUnavailable,
    ] {
        let mut cleared = false;

        let result = super::apply_refresh_tokens(
            "untrusted-access",
            "untrusted-refresh",
            Err(error),
            |_, _| panic!("untrusted tokens must not be saved"),
            || cleared = true,
        );

        assert!(cleared, "verification failure: {error:?}");
        assert!(
            matches!(
                result,
                Err(RefreshError::SessionEnded {
                    reason: SessionEndReason::InvalidAccessToken,
                    ..
                })
            ),
            "verification failure: {error:?}"
        );
    }
}
