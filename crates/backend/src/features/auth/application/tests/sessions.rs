//! Auth session application tests.

use cheenhub_contracts::rest::{LoginRequest, RegisterRequest, SessionDeviceKind};

use super::state;
use crate::features::auth::application::{
    active_sessions, login_with_user_agent, me, register_with_user_agent,
    revoke_current_user_session, revoke_current_user_sessions,
};

const DESKTOP_CHROME_USER_AGENT: &str = concat!(
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 ",
    "(KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36"
);

#[tokio::test]
async fn active_sessions_show_current_session_and_parse_user_agent() {
    let state = state();
    let auth = register_with_user_agent(
        &state,
        RegisterRequest {
            nickname: "session_user".to_owned(),
            email: "session-user@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
        Some(DESKTOP_CHROME_USER_AGENT.to_owned()),
    )
    .await
    .expect("registration should succeed");

    let response = active_sessions(&state, &auth.access_token)
        .await
        .expect("active sessions should load");

    assert_eq!(response.sessions.len(), 1);
    let session = &response.sessions[0];
    assert!(session.current);
    assert_eq!(
        session.user_agent.as_deref(),
        Some(DESKTOP_CHROME_USER_AGENT)
    );
    assert_eq!(session.client.device_kind, SessionDeviceKind::Desktop);
    assert_eq!(session.client.os_name, "Linux");
    assert_eq!(session.client.browser_name, "Chrome");
}

#[tokio::test]
async fn revoke_specific_session_invalidates_that_access_token_and_keeps_current() {
    let state = state();
    let first_auth = register_with_user_agent(
        &state,
        RegisterRequest {
            nickname: "target_session".to_owned(),
            email: "target-session@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
        Some(DESKTOP_CHROME_USER_AGENT.to_owned()),
    )
    .await
    .expect("registration should succeed");
    let current_auth = login_with_user_agent(
        &state,
        LoginRequest {
            email: "target-session@example.com".to_owned(),
            password: "password123".to_owned(),
        },
        Some(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) Mobile/15E148 Safari/604.1"
                .to_owned(),
        ),
    )
    .await
    .expect("login should succeed");
    let sessions = active_sessions(&state, &current_auth.access_token)
        .await
        .expect("active sessions should load");
    let revoked_session_id = sessions
        .sessions
        .iter()
        .find(|session| !session.current)
        .map(|session| session.id.clone())
        .expect("other session should be present");

    revoke_current_user_session(&state, &current_auth.access_token, &revoked_session_id)
        .await
        .expect("specific session revoke should succeed");

    let revoked_user = me(&state, &first_auth.access_token).await;
    assert!(revoked_user.is_err());
    me(&state, &current_auth.access_token)
        .await
        .expect("current session should remain active");
}

#[tokio::test]
async fn revoke_all_sessions_invalidates_current_access_token() {
    let state = state();
    let auth = register_with_user_agent(
        &state,
        RegisterRequest {
            nickname: "all_sessions".to_owned(),
            email: "all-sessions@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
        Some(DESKTOP_CHROME_USER_AGENT.to_owned()),
    )
    .await
    .expect("registration should succeed");

    revoke_current_user_sessions(&state, &auth.access_token)
        .await
        .expect("all session revoke should succeed");

    let current_user = me(&state, &auth.access_token).await;
    assert!(current_user.is_err());
}
