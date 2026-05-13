//! Password change application tests.

use cheenhub_contracts::rest::{ChangeCurrentUserPasswordRequest, LoginRequest};

use super::{login, registered_user, state, state_with_mailer};
use crate::features::auth::application::change_current_user_password;

#[tokio::test]
async fn change_current_user_password_changes_password_and_sends_email() {
    let (state, mailer) = state_with_mailer();
    let auth = registered_user(&state, "profile_password", "profile-password@example.com").await;

    change_current_user_password(
        &state,
        &auth.access_token,
        ChangeCurrentUserPasswordRequest {
            current_password: "password123".to_owned(),
            new_password: "new-password123".to_owned(),
            new_password_confirmation: "new-password123".to_owned(),
        },
    )
    .await
    .expect("password change should succeed");

    let old_login = login(
        &state,
        LoginRequest {
            email: "profile-password@example.com".to_owned(),
            password: "password123".to_owned(),
        },
    )
    .await;
    assert!(old_login.is_err());

    let new_login = login(
        &state,
        LoginRequest {
            email: "profile-password@example.com".to_owned(),
            password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("new password should work");
    assert_eq!(new_login.user.email, "profile-password@example.com");

    let sent = mailer.password_changed();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "profile-password@example.com");
}

#[tokio::test]
async fn change_current_user_password_rejects_invalid_current_password() {
    let state = state();
    let auth = registered_user(&state, "bad_current", "bad-current@example.com").await;

    let result = change_current_user_password(
        &state,
        &auth.access_token,
        ChangeCurrentUserPasswordRequest {
            current_password: "wrong-password".to_owned(),
            new_password: "new-password123".to_owned(),
            new_password_confirmation: "new-password123".to_owned(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn passwordless_user_can_set_first_password_without_current_password() {
    let (state, mailer) = state_with_mailer();
    let auth = super::google_only_user(&state).await;

    change_current_user_password(
        &state,
        &auth.access_token,
        ChangeCurrentUserPasswordRequest {
            current_password: String::new(),
            new_password: "new-password123".to_owned(),
            new_password_confirmation: "new-password123".to_owned(),
        },
    )
    .await
    .expect("passwordless account should set first password");

    let login = login(
        &state,
        LoginRequest {
            email: "google-only@example.com".to_owned(),
            password: "new-password123".to_owned(),
        },
    )
    .await
    .expect("new password should work");
    assert!(login.user.has_password);
    assert_eq!(mailer.password_changed().len(), 1);
}
