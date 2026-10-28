//! Проверки одноразового браузерного входа в desktop-клиент.

use cheenhub_contracts::rest::{BrowserDesktopAuthPollResponse, BrowserDesktopAuthRequest};

use super::support::{registered_user, state};
use crate::features::auth::application::{
    approve_browser_handoff, cancel_browser_handoff, complete_browser_handoff,
    poll_browser_handoff, start_browser_handoff,
};

#[tokio::test]
async fn browser_handoff_requires_approval_then_creates_independent_session_once() {
    let state = state();
    let browser = registered_user(&state, "browser_user", "browser@example.com").await;
    let start = start_browser_handoff(&state).await.expect("start");
    let request = BrowserDesktopAuthRequest {
        attempt_id: start.attempt_id,
        poll_secret: start.poll_secret.clone(),
    };

    assert_eq!(
        poll_browser_handoff(&state, request.clone())
            .await
            .expect("pending"),
        BrowserDesktopAuthPollResponse::Pending
    );

    approve_browser_handoff(&state, start.attempt_id, &browser.access_token)
        .await
        .expect("approve");
    assert_eq!(
        poll_browser_handoff(&state, request.clone())
            .await
            .expect("ready"),
        BrowserDesktopAuthPollResponse::Ready
    );

    let desktop =
        complete_browser_handoff(&state, request.clone(), Some("CheenHub/Linux".to_owned()))
            .await
            .expect("claim");
    assert_eq!(desktop.user.id, browser.user.id);
    assert_ne!(desktop.refresh_token, browser.refresh_token);
    assert_ne!(desktop.access_token, browser.access_token);
    assert!(
        complete_browser_handoff(&state, request, None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn browser_handoff_does_not_accept_unapproved_or_unauthorized_attempts() {
    let state = state();
    let start = start_browser_handoff(&state).await.expect("start");
    let request = BrowserDesktopAuthRequest {
        attempt_id: start.attempt_id,
        poll_secret: start.poll_secret.clone(),
    };

    assert!(
        complete_browser_handoff(&state, request.clone(), None)
            .await
            .is_err()
    );
    assert!(
        approve_browser_handoff(&state, start.attempt_id, "invalid")
            .await
            .is_err()
    );
    assert!(
        poll_browser_handoff(
            &state,
            BrowserDesktopAuthRequest {
                attempt_id: start.attempt_id,
                poll_secret: "incorrect-secret".to_owned(),
            }
        )
        .await
        .is_err()
    );
    assert_eq!(
        poll_browser_handoff(&state, request)
            .await
            .expect("still pending"),
        BrowserDesktopAuthPollResponse::Pending
    );
}

#[tokio::test]
async fn cancelling_browser_handoff_blocks_late_approval_and_claim() {
    let state = state();
    let browser = registered_user(&state, "cancel_user", "cancel@example.com").await;
    let start = start_browser_handoff(&state).await.expect("start");
    let request = BrowserDesktopAuthRequest {
        attempt_id: start.attempt_id,
        poll_secret: start.poll_secret,
    };

    cancel_browser_handoff(&state, request.clone())
        .await
        .expect("cancel");

    assert!(
        approve_browser_handoff(&state, start.attempt_id, &browser.access_token)
            .await
            .is_err()
    );
    assert!(
        complete_browser_handoff(&state, request, None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn expired_browser_handoff_cannot_be_approved() {
    use chrono::{Duration, Utc};

    let state = state();
    let user = registered_user(&state, "expired_user", "expired@example.com").await;
    let attempt = start_browser_handoff(&state).await.expect("start");
    let after_expiry = Utc::now() + Duration::minutes(6);
    assert!(
        state
            .auth_store
            .find_browser_handoff(&attempt.attempt_id, after_expiry)
            .await
            .expect("find")
            .is_none()
    );
    assert!(
        !state
            .auth_store
            .approve_browser_handoff(
                &attempt.attempt_id,
                &uuid::Uuid::parse_str(&user.user.id).expect("user id"),
                after_expiry,
            )
            .await
            .expect("approval result")
    );
}

#[tokio::test]
async fn concurrent_browser_handoff_claims_have_one_winner() {
    let state = state();
    let browser = registered_user(&state, "race_user", "race@example.com").await;
    let start = start_browser_handoff(&state).await.expect("start");
    approve_browser_handoff(&state, start.attempt_id, &browser.access_token)
        .await
        .expect("approve");
    let request = BrowserDesktopAuthRequest {
        attempt_id: start.attempt_id,
        poll_secret: start.poll_secret,
    };

    let (a, b) = tokio::join!(
        complete_browser_handoff(&state, request.clone(), None),
        complete_browser_handoff(&state, request, None),
    );

    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
}
