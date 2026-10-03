//! Сценарии ротации и отзыва refresh-токенов.

use cheenhub_contracts::rest::{LogoutRequest, RefreshRequest};
use chrono::{Duration, Utc};

use super::support::{register_test_session, registered_user, state};
use crate::features::auth::application::{logout, refresh_with_user_agent};
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;

#[tokio::test]
async fn concurrent_refresh_preserves_winning_rotation() {
    let state = state();
    let auth = registered_user(&state, "refresh_race", "refresh-race@example.com").await;
    let request = RefreshRequest {
        refresh_token: auth.refresh_token.clone(),
    };

    let (first, second) = tokio::join!(
        refresh_with_user_agent(&state, request.clone(), None),
        refresh_with_user_agent(&state, request, None),
    );
    let (winner, loser) = match (first, second) {
        (Ok(winner), Err(loser)) | (Err(loser), Ok(winner)) => (winner, loser),
        outcome => panic!("expected one refresh winner and one loser, got {outcome:?}"),
    };

    assert!(matches!(loser, AuthError::RefreshRotationInProgress(_)));
    refresh_with_user_agent(
        &state,
        RefreshRequest {
            refresh_token: winner.refresh_token,
        },
        None,
    )
    .await
    .expect("winning refresh chain should remain active");
}

#[tokio::test]
async fn explicitly_revoked_refresh_is_not_reported_as_reuse() {
    let state = state();
    let auth = registered_user(&state, "revoked_refresh", "revoked-refresh@example.com").await;
    let realtime_disconnect = register_test_session(&state, &auth).await;
    logout(
        &state,
        LogoutRequest {
            refresh_token: auth.refresh_token.clone(),
        },
    )
    .await
    .expect("logout should revoke session");
    assert_eq!(
        *realtime_disconnect.borrow(),
        Some(crate::realtime::hub::DisconnectReason::AuthSessionRevoked)
    );

    let error = refresh_with_user_agent(
        &state,
        RefreshRequest {
            refresh_token: auth.refresh_token,
        },
        None,
    )
    .await
    .expect_err("revoked refresh must fail");

    assert!(matches!(
        error,
        AuthError::RefreshRejected {
            reason: crate::features::auth::error::RefreshRejection::SessionRevoked,
            ..
        }
    ));
}

#[tokio::test]
async fn refresh_replay_outside_grace_revokes_session() {
    let state = state();
    let auth = registered_user(&state, "refresh_replay", "refresh-replay@example.com").await;
    let rotated = refresh_with_user_agent(
        &state,
        RefreshRequest {
            refresh_token: auth.refresh_token.clone(),
        },
        None,
    )
    .await
    .expect("first refresh should rotate token");
    let realtime_disconnect = register_test_session(&state, &auth).await;
    let detection_time = Utc::now() + Duration::seconds(10);

    let error = crate::features::auth::application::refresh::classify_inactive_token_for_test(
        &state,
        &refresh_token::hash(&auth.refresh_token),
        detection_time,
        detection_time - Duration::seconds(5),
    )
    .await
    .expect("reuse detection should return an auth rejection");
    assert!(matches!(
        error,
        AuthError::RefreshRejected {
            reason: crate::features::auth::error::RefreshRejection::Reused,
            ..
        }
    ));
    assert_eq!(
        *realtime_disconnect.borrow(),
        Some(crate::realtime::hub::DisconnectReason::AuthSessionRevoked)
    );

    let error = refresh_with_user_agent(
        &state,
        RefreshRequest {
            refresh_token: rotated.refresh_token,
        },
        None,
    )
    .await
    .expect_err("reuse must revoke the winning refresh chain");
    assert!(matches!(
        error,
        AuthError::RefreshRejected {
            reason: crate::features::auth::error::RefreshRejection::SessionRevoked,
            ..
        }
    ));
}
