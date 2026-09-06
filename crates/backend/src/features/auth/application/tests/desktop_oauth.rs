//! Тесты передачи Google OAuth настольному приложению.

use cheenhub_contracts::rest::*;
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::{registered_user, state};
use crate::features::auth::application::desktop_oauth::finish_identity;
use crate::features::auth::application::google::GoogleIdentity;
use crate::features::auth::application::{
    GoogleCallbackOutcome, cancel_desktop_oauth, complete_google_oauth, google_oauth_callback,
    poll_desktop_oauth, register_with_google_oauth, start_desktop_oauth,
};
use crate::features::auth::domain::{DesktopOAuthAttempt, DesktopOAuthIdentity};
use crate::features::auth::security::refresh_token;
use crate::state::AppState;

async fn start(state: &AppState) -> GoogleDesktopAuthStartResponse {
    start_desktop_oauth(
        state,
        None,
        OAuthStartRequest {
            flow: OAuthFlow::Login,
        },
    )
    .await
    .expect("desktop start")
}

fn request(start: &GoogleDesktopAuthStartResponse) -> GoogleDesktopAuthRequest {
    GoogleDesktopAuthRequest {
        attempt_id: start.attempt_id,
        poll_secret: start.poll_secret.clone(),
    }
}

fn state_value(start: &GoogleDesktopAuthStartResponse) -> String {
    url::Url::parse(&start.authorization_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned()
}

async fn ready(state: &AppState, start: &GoogleDesktopAuthStartResponse, email: &str) {
    let oauth_state = state
        .auth_store
        .consume_oauth_state(&refresh_token::hash(&state_value(start)), Utc::now())
        .await
        .unwrap()
        .unwrap();
    assert!(
        finish_identity(
            state,
            start.attempt_id,
            &oauth_state,
            GoogleIdentity {
                subject: format!("subject-{email}"),
                email: email.to_owned(),
                display_name: Some("Google User".to_owned()),
            }
        )
        .await
        .unwrap()
    );
}

async fn complete(
    state: &AppState,
    start: &GoogleDesktopAuthStartResponse,
) -> Result<OAuthCompleteResponse, crate::features::auth::error::AuthError> {
    complete_google_oauth(
        state,
        OAuthCompleteRequest {
            handoff_code: start.poll_secret.clone(),
        },
        None,
    )
    .await
}

#[tokio::test]
async fn pending_secret_is_not_browser_authorization_material() {
    let state = state();
    let start = start(&state).await;
    assert!(!start.authorization_url.contains(&start.poll_secret));
    assert!(
        !start
            .authorization_url
            .contains(&start.attempt_id.to_string())
    );
    assert_eq!(start.expires_in_seconds, 300);
    assert_eq!(start.poll_interval_seconds, 2);
    assert_eq!(
        poll_desktop_oauth(&state, request(&start)).await.unwrap(),
        GoogleDesktopAuthPollResponse::Pending
    );
    let mut wrong = request(&start);
    wrong.poll_secret = "wrong".to_owned();
    assert!(poll_desktop_oauth(&state, wrong.clone()).await.is_err());
    cancel_desktop_oauth(&state, wrong).await.unwrap();
    assert_eq!(
        poll_desktop_oauth(&state, request(&start)).await.unwrap(),
        GoogleDesktopAuthPollResponse::Pending
    );
    assert!(complete(&state, &start).await.is_err());
}

#[tokio::test]
async fn desktop_login_creates_session_only_in_completing_client() {
    let state = state();
    let existing = registered_user(&state, "desktop_login", "desktop@example.com").await;
    let start = start(&state).await;
    ready(&state, &start, &existing.user.email).await;
    assert_eq!(
        poll_desktop_oauth(&state, request(&start)).await.unwrap(),
        GoogleDesktopAuthPollResponse::Ready
    );
    assert!(
        state
            .auth_store
            .find_oauth_account_by_subject("google", "subject-desktop@example.com")
            .await
            .unwrap()
            .is_none()
    );
    match complete(&state, &start).await.unwrap() {
        OAuthCompleteResponse::Authenticated { auth } => assert_eq!(auth.user.id, existing.user.id),
        _ => panic!("expected authenticated"),
    }
    assert!(complete(&state, &start).await.is_err());
    assert!(poll_desktop_oauth(&state, request(&start)).await.is_err());
}

#[tokio::test]
async fn desktop_registration_retains_separate_consent() {
    let state = state();
    let start = start(&state).await;
    ready(&state, &start, "newdesktop@example.com").await;
    let OAuthCompleteResponse::RegistrationRequired {
        registration_token,
        email,
        ..
    } = complete(&state, &start).await.unwrap()
    else {
        panic!("expected registration");
    };
    assert_eq!(email, "newdesktop@example.com");
    assert_ne!(registration_token, start.poll_secret);
    let mut registration = OAuthRegistrationRequest {
        registration_token,
        nickname: "new_desktop".to_owned(),
        accepts_terms: true,
        accepts_personal_data: false,
    };
    assert!(
        register_with_google_oauth(&state, registration.clone(), None)
            .await
            .is_err()
    );
    registration.accepts_personal_data = true;
    assert_eq!(
        register_with_google_oauth(&state, registration, None)
            .await
            .unwrap()
            .user
            .email,
        email
    );
}

#[tokio::test]
async fn desktop_link_requires_bearer_and_defers_mutation_until_claim() {
    let state = state();
    assert!(
        start_desktop_oauth(
            &state,
            None,
            OAuthStartRequest {
                flow: OAuthFlow::Link
            }
        )
        .await
        .is_err()
    );
    let auth = registered_user(&state, "desktop_link", "linklocal@example.com").await;
    let start = start_desktop_oauth(
        &state,
        Some(&auth.access_token),
        OAuthStartRequest {
            flow: OAuthFlow::Link,
        },
    )
    .await
    .unwrap();
    ready(&state, &start, "linkgoogle@example.com").await;
    let id = Uuid::parse_str(&auth.user.id).unwrap();
    assert!(
        state
            .auth_store
            .find_oauth_account_for_user("google", &id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        complete(&state, &start).await.unwrap(),
        OAuthCompleteResponse::Linked { .. }
    ));
    assert!(
        state
            .auth_store
            .find_oauth_account_for_user("google", &id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn cancelling_ready_link_never_changes_account() {
    let state = state();
    let auth = registered_user(&state, "cancel_link", "cancel-local@example.com").await;
    let start = start_desktop_oauth(
        &state,
        Some(&auth.access_token),
        OAuthStartRequest {
            flow: OAuthFlow::Link,
        },
    )
    .await
    .unwrap();
    ready(&state, &start, "cancel-google@example.com").await;
    cancel_desktop_oauth(&state, request(&start)).await.unwrap();
    assert_eq!(
        poll_desktop_oauth(&state, request(&start)).await.unwrap(),
        GoogleDesktopAuthPollResponse::Cancelled
    );
    assert!(complete(&state, &start).await.is_err());
    assert!(
        state
            .auth_store
            .find_oauth_account_for_user("google", &Uuid::parse_str(&auth.user.id).unwrap())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn google_error_consumes_state_and_stays_in_desktop_flow() {
    let state = state();
    let start = start(&state).await;
    let value = state_value(&start);
    let outcome = google_oauth_callback(
        &state,
        None,
        Some(value.clone()),
        Some("sensitive-provider-error".to_owned()),
    )
    .await;
    assert!(matches!(
        outcome,
        GoogleCallbackOutcome::Desktop { success: false }
    ));
    let status = poll_desktop_oauth(&state, request(&start)).await.unwrap();
    assert!(
        matches!(status, GoogleDesktopAuthPollResponse::Failed { message } if !message.contains("sensitive-provider-error"))
    );
    assert!(
        state
            .auth_store
            .consume_oauth_state(&refresh_token::hash(&value), Utc::now())
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        google_oauth_callback(&state, None, Some(value), None).await,
        GoogleCallbackOutcome::Desktop { success: false }
    ));
}

#[tokio::test]
async fn cancelled_callback_does_not_become_browser_login() {
    let state = state();
    let start = start(&state).await;
    cancel_desktop_oauth(&state, request(&start)).await.unwrap();
    assert!(matches!(
        google_oauth_callback(
            &state,
            Some("unused".to_owned()),
            Some(state_value(&start)),
            None
        )
        .await,
        GoogleCallbackOutcome::Desktop { success: false }
    ));
    assert_eq!(
        poll_desktop_oauth(&state, request(&start)).await.unwrap(),
        GoogleDesktopAuthPollResponse::Cancelled
    );
}

#[tokio::test]
async fn duplicate_callback_does_not_overwrite_ready_result() {
    let state = state();
    let start = start(&state).await;
    ready(&state, &start, "duplicate@example.com").await;
    assert!(matches!(
        google_oauth_callback(
            &state,
            None,
            Some(state_value(&start)),
            Some("error".to_owned())
        )
        .await,
        GoogleCallbackOutcome::Desktop { success: false }
    ));
    assert_eq!(
        poll_desktop_oauth(&state, request(&start)).await.unwrap(),
        GoogleDesktopAuthPollResponse::Ready
    );
}

#[tokio::test]
async fn expired_attempt_never_accepts_identity_or_wrong_secret() {
    let state = state();
    let now = Utc::now();
    let oauth_state_id = state
        .auth_store
        .insert_oauth_state(
            "expired_state".to_owned(),
            "nonce".to_owned(),
            "login".to_owned(),
            None,
            now,
            now + Duration::minutes(5),
        )
        .await
        .unwrap();
    let attempt_id = Uuid::new_v4();
    state
        .auth_store
        .insert_desktop_oauth_attempt(DesktopOAuthAttempt {
            id: attempt_id,
            oauth_state_id,
            secret_hash: refresh_token::hash("secret"),
            expires_at: now - Duration::seconds(1),
        })
        .await
        .unwrap();
    assert_eq!(
        poll_desktop_oauth(
            &state,
            GoogleDesktopAuthRequest {
                attempt_id,
                poll_secret: "secret".to_owned()
            }
        )
        .await
        .unwrap(),
        GoogleDesktopAuthPollResponse::Expired
    );
    assert!(
        poll_desktop_oauth(
            &state,
            GoogleDesktopAuthRequest {
                attempt_id,
                poll_secret: "wrong".to_owned()
            }
        )
        .await
        .is_err()
    );
    assert!(
        !state
            .auth_store
            .finish_desktop_oauth_attempt(
                &attempt_id,
                "desktop_login".to_owned(),
                None,
                DesktopOAuthIdentity {
                    subject: "subject".to_owned(),
                    email: "expired@example.com".to_owned(),
                    display_name: None
                },
                now
            )
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn concurrent_completions_have_one_winner() {
    let state = state();
    let start = start(&state).await;
    ready(&state, &start, "race@example.com").await;
    let (first, second) = tokio::join!(complete(&state, &start), complete(&state, &start));
    assert_ne!(first.is_ok(), second.is_ok());
}

#[tokio::test]
async fn cancel_and_completion_have_mutually_exclusive_results() {
    let state = state();
    let start = start(&state).await;
    ready(&state, &start, "cancelrace@example.com").await;
    let secret_hash = refresh_token::hash(&start.poll_secret);
    let (cancelled, completed) = tokio::join!(
        state
            .auth_store
            .cancel_desktop_oauth_attempt(&start.attempt_id, &secret_hash, Utc::now()),
        complete(&state, &start),
    );
    assert_ne!(cancelled.unwrap(), completed.is_ok());
}
